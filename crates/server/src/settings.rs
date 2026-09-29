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
//! mcp_headers:
//!   docs:
//!     Authorization: Bearer ...
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
//! session_path:
//!   - /home/you/.local/bin
//! ignored_comments:
//!   - author: coderabbitai
//!     body: billing
//! mcp_servers:
//!   - name: docs
//!     url: https://mcp.example.com/docs
//!     headers:
//!       - Authorization
//! instructions: |
//!   Prefer the smallest change that does the job.
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

use std::collections::BTreeMap;
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
///
/// What everything else Verkstead writes under the Data Directory is written as
/// too — see [`write_atomically`], whose one caller outside this module is the
/// record a container is swept by.
pub(crate) const ORDINARY_MODE: u32 = 0o644;

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
    ///
    /// **Cleared means nothing set at all, rather than no token.** There is
    /// more than one secret in this file now — the Windows session account's
    /// password is kept beside the token, written once by an elevated verb and
    /// never again — so emptying the file whenever the token is absent would
    /// take that password away the first time somebody cleared a token, and
    /// leave a machine whose sessions cannot start with nothing on the settings
    /// page to say why. What a caller hands in is the whole of what the file
    /// will hold, which is why the two ways of building one are both `with_` on
    /// the secrets already there — see [`Secrets::with_token`].
    pub fn save_secrets(&self, secrets: &Secrets) -> std::io::Result<()> {
        let text = match secrets.anything_set() {
            true => yaml(secrets)?,
            false => String::new(),
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
///
/// **Reachable from the rest of the crate**, because the settings files are not
/// the only thing under the Data Directory a half of would be worse than
/// nothing. The record a Conversation's entries are swept by is one: it is
/// read by a server that did not write it, and one that will not parse is a
/// boundary nothing will ever take off the human's own directories — see
/// [`crate::sandbox::granting::remembering`]. The Workbench Key is the other,
/// and wants exactly what a secrets file wants: a credential in a file of its
/// own in the same directory, at the same mode — see [`crate::key`].
pub(crate) fn write_atomically(path: &Path, text: &str, mode: u32) -> std::io::Result<()> {
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

/// What `secrets.yaml` says. Flat, because what is in it are secrets rather
/// than a structure.
///
/// Unknown keys are ignored rather than refused: the human hand-edits this file,
/// and a key from a later Verkstead — or a comment they left as a key by mistake
/// — is not worth taking a session's credentials away over.
///
/// **Three secrets now, and they are written by different hands.** The token is
/// the settings page's, typed and retyped and cleared; the session account's
/// password is an elevated verb's, written once when the account is made and
/// read by every Windows session after that; and the header values the declared
/// MCP servers are spoken to with are the settings page's again, from another
/// section of it. None of them may take another away, which is why there is no
/// constructor here that says what the whole file is — only
/// [`Secrets::with_token`], [`Secrets::with_session_account_password`] and
/// [`Secrets::with_mcp_headers`], each of which is the secrets that are already
/// there with one of them replaced.
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

    /// And the password of the local account a Windows session runs as, or
    /// `None` on a machine where the elevated verb has not been run — which is
    /// every machine that is not a Windows one.
    ///
    /// Here rather than in a file of its own because this is the file that is
    /// already written 0600 and already never in a sandbox: what
    /// `CreateProcessWithLogonW` needs is a password, and a password Verkstead
    /// keeps is a secret whatever it opens — see
    /// [`crate::sandbox::account`], which is what generates one and what reads
    /// it back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_account_password: Option<String>,

    /// And the header values each declared MCP server is spoken to with, by
    /// the server's name and then by the headers: `{"docs": {"Authorization":
    /// "Bearer ..."}}`, and empty on an installation that has declared none.
    ///
    /// **Here because every one of them is a secret.** A header is what an
    /// API key or a bearer token is sent in, and static headers are the only
    /// authentication a declaration has — OAuth was turned down in the
    /// grilling this was settled in, there being no browser at three in the
    /// morning. So there is no plain kind: a value typed into this section
    /// goes where the GitHub token goes, and what comes back to the page is
    /// the names and nothing else.
    ///
    /// **The names are in `config.yaml` and the values are here**, which is
    /// the one thing in either file said in both. A header name is nobody's
    /// secret and it is part of the declaration — what the page draws, and
    /// what says which headers a server has at all — so it is written where
    /// the declaration is. This holds what may not be read back, and holds it
    /// under the name that declared it: a header the declaration does not
    /// name is not sent, and a server that is no longer declared has nothing
    /// left here at all — see [`Secrets::with_mcp_headers`].
    ///
    /// A map rather than the declaration's own order, because the order of
    /// headers on a request is nobody's business and a map is what is read
    /// back by name. Sorted, so that what a save writes is the same file
    /// twice over.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    mcp_headers: BTreeMap<String, BTreeMap<String, String>>,
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
            session_account_password: secrets.session_account_password.and_then(blank_is_nothing),
            // And the headers with the blanks taken out of them: a value
            // somebody emptied by hand is a header nothing is sent in, and a
            // server left holding none of them is not declared here at all —
            // which is what a save writes when the last of them is cleared, so
            // that a file written by hand and a file written by the page read
            // alike.
            mcp_headers: headers_kept(secrets.mcp_headers),
        })
    }

    /// These secrets with the GitHub token replaced: `token` as it was typed,
    /// or `None` where the human cleared it.
    ///
    /// On the secrets that are already there rather than on nothing, because
    /// what a save writes is the whole file — see
    /// [`Settings::save_secrets`] — and the other secret in it is not the
    /// settings page's to take away.
    ///
    /// Whitespace is nothing, as it is on the way in — see [`blank_is_nothing`].
    /// A token pasted with the newline that came with it is the ordinary case,
    /// and one that was only spaces is a cleared field spelled another way.
    pub fn with_token(&self, token: Option<String>) -> Secrets {
        Secrets {
            github_token: token.and_then(blank_is_nothing),
            ..self.clone()
        }
    }

    /// And these secrets with the session account's password replaced, which is
    /// the elevated verb's write and nobody else's.
    ///
    /// `None` is the account having been taken away: the verb that deletes one
    /// clears the password in the same breath, so a Data Directory whose
    /// account is gone does not go on holding the password of an account that
    /// is not there.
    pub fn with_session_account_password(&self, password: Option<String>) -> Secrets {
        Secrets {
            session_account_password: password.and_then(blank_is_nothing),
            ..self.clone()
        }
    }

    /// And these secrets with the header values of every server in `declared`
    /// written down — each header's own action applied to what is kept for it,
    /// and the headers of every server *not* in `declared` gone.
    ///
    /// Which is what a save of the MCP servers section writes. `declared` is
    /// that section as it is to stand: one entry per server, holding the header
    /// names that server declares and what is to become of each of their values
    /// — see [`HeaderValue`].
    ///
    /// **A server left out loses its headers**, which is how a declaration
    /// deleted on the page takes its secrets with it: the section sends the
    /// whole list, so a server that is not in it is one nobody declares any
    /// more, and declaring that name again starts with nothing kept for it.
    ///
    /// **A header with no value is no header.** One cleared, one set to
    /// whitespace and one never typed come to the same nothing, and nothing is
    /// what is written down — the name stays declared in `config.yaml`, and
    /// there is simply nothing here to send in it. A server whose headers all
    /// come to nothing is dropped, so that an empty map never reaches the file.
    pub fn with_mcp_headers(&self, declared: &[(String, Vec<(String, HeaderValue)>)]) -> Secrets {
        let written = declared
            .iter()
            .map(|(server, headers)| {
                let kept = self.mcp_headers.get(server);

                let headers = headers
                    .iter()
                    .filter_map(|(header, value)| {
                        let value = match value {
                            // What is kept, which is what a value box left
                            // blank means: a save correcting a URL is not one
                            // that takes a key away.
                            HeaderValue::Keep => kept.and_then(|kept| kept.get(header)).cloned(),
                            HeaderValue::Set(typed) => blank_is_nothing(typed.clone()),
                            HeaderValue::Clear => None,
                        };

                        Some((header.clone(), value?))
                    })
                    .collect::<BTreeMap<_, _>>();

                (server.clone(), headers)
            })
            .filter(|(_, headers)| !headers.is_empty())
            .collect();

        Secrets {
            mcp_headers: written,
            ..self.clone()
        }
    }

    /// The configured GitHub token, or `None` where there is none.
    pub fn github_token(&self) -> Option<&str> {
        self.github_token.as_deref()
    }

    /// And the session account's password, or `None` where the elevated verb
    /// has not been run for this Data Directory.
    pub fn session_account_password(&self) -> Option<&str> {
        self.session_account_password.as_deref()
    }

    /// And what is sent in `header` to the server declared as `server`, or
    /// `None` where nothing is kept for it — a header declared and never given
    /// a value, or one that was cleared.
    ///
    /// By name both ways round, because that is what this file holds: the
    /// declaration says which headers a server has and in what order, and this
    /// says what goes in each of them — see [`Config::attached_among`], which
    /// is where the two are put together.
    pub fn mcp_header(&self, server: &str, header: &str) -> Option<&str> {
        self.mcp_headers
            .get(server)?
            .get(header)
            .map(String::as_str)
    }

    /// Whether anything at all is configured here, which is what says a save
    /// writes a file rather than empties one.
    ///
    /// Every field, said once: a secret added to this struct and left out of
    /// here would be a secret that a save quietly threw away.
    fn anything_set(&self) -> bool {
        let Secrets {
            github_token,
            session_account_password,
            mcp_headers,
        } = self;

        github_token.is_some() || session_account_password.is_some() || !mcp_headers.is_empty()
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
    /// installation: a flat list in the grammar `--sandbox-bind` takes, which is
    /// `/abs/path` for a bind every sandbox gets.
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

    /// And the directories a session's `PATH` is composed with **ahead of the
    /// server's own entries**: where Verkstead has installed something, so that
    /// a harness it put in `~/.local/bin` is the one a session finds rather than
    /// whatever older copy the distribution's packages hold.
    ///
    /// Read at startup beside the server's own `PATH` and held for the run, and
    /// by the same three rules — see [`crate::sandbox::composed`]. The one key
    /// in this file Verkstead writes for itself: an install that lands in a
    /// directory appends it here, and the next probe and the next session both
    /// see it without a restart.
    ///
    /// **Not a field on the settings page**, which is the settings module's
    /// *told, not found* rule kept rather than broken: what is written here is
    /// a directory the human ticked an install into, or one they typed by hand.
    #[serde(
        default,
        deserialize_with = "rows_written",
        skip_serializing_if = "Vec::is_empty"
    )]
    session_path: Vec<String>,

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

    /// And the MCP servers declared for this installation: a name and a URL
    /// each, spoken to over HTTP, out of which a Conversation attaches the ones
    /// its sessions are launched with.
    ///
    /// Declared here rather than beside a Conversation because a declaration is
    /// a thing Verkstead is *told*, like the binds above it and the
    /// instructions below: said once for the machine, read at the moment it is
    /// needed, and referred to by name from wherever it is attached. What a
    /// Conversation holds is that name.
    ///
    /// Read the way everything else in this file is, and refused the way the
    /// rules above it are. An absent key, an absent file and one nothing can
    /// parse all mean no servers, and an entry missing either half is dropped
    /// as it is read — a declaration with no name is one nothing could refer
    /// to, and one with no URL reaches nothing. What is *refused* rather than
    /// dropped is a save from the settings page, so that a name nobody could
    /// use is said at the moment somebody types it — see [`trouble_among`].
    #[serde(
        default,
        deserialize_with = "servers_written",
        skip_serializing_if = "Vec::is_empty"
    )]
    mcp_servers: Vec<McpServer>,

    /// And the one text every session is given, whatever harness runs it: what
    /// a human would have put in their own global `CLAUDE.md`, said once here
    /// because a Built Root holds none of the account's own files.
    ///
    /// One text for the whole installation rather than one per Agent Profile,
    /// which is why it is in this file rather than beside a Profile in the
    /// store: it is a thing Verkstead is *told*, like the author above it and
    /// the binds beside it, and it is read afresh the moment a session needs it
    /// — a change on the settings page reaches the next session, and a running
    /// one keeps what it started with.
    ///
    /// Nothing here can be wrong. It is a paragraph of somebody's prose, so
    /// there is nothing to parse and nothing to refuse: an absent key, an
    /// absent file and one nothing can read all mean an empty text, which is a
    /// session told nothing beyond what its Repo carries. An `Option` for that
    /// reason and written away when it is nothing, so that clearing the box
    /// takes the key out of the file rather than leaving an empty one behind.
    ///
    /// Kept exactly as it was typed, which is the one thing in this file not
    /// put through [`blank_is_nothing`] — see [`prose_written`]. What the
    /// setting is for is the words a harness reads, and a text a save quietly
    /// reshaped would be one the human did not write.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
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
            session_path: entries_written(config.session_path),
            ignored_comments: rules_kept(config.ignored_comments),
            // And the declarations, with the blanks taken out of each and the
            // ones that came to nothing dropped — see [`servers_kept`].
            mcp_servers: servers_kept(config.mcp_servers),
            // Whitespace and all, bar a text that is nothing but whitespace —
            // see [`prose_written`].
            instructions: config.instructions.and_then(prose_written),
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
        ignored_comments: Vec<IgnoreRule>,
        mcp_servers: Vec<McpServer>,
        instructions: String,
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
            // And nothing at all for the key the page has no field for. What
            // keeps a save from taking it away is [`Config::keeping_session_path`],
            // said at the endpoint over what the file already holds — the same
            // shape [`Secrets::with_token`] keeps for the password beside the
            // token, and for the same reason: a page told about one thing has no
            // business rewriting another.
            session_path: Vec::new(),
            // Whole, and not put through the reading half's own drop above: what
            // reaches here has already been through [`IgnoreRule::trouble`] at
            // the endpoint, which refuses the rule the reading merely skips —
            // and dropping one here would be a save that quietly wrote fewer
            // rules than the page sent.
            ignored_comments,
            // And the declared servers, whole and undropped, for the reason the
            // rules above are: what reaches here has already been through
            // [`trouble_among`] at the endpoint, which refuses what the reading
            // half would merely skip.
            mcp_servers,
            // As it was typed, and away altogether where the box was cleared:
            // there is nothing to configure in an empty text, and a key holding
            // one would read as a setting somebody made.
            instructions: prose_written(instructions),
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

    /// And the directories a session's `PATH` leads with, in the order they
    /// were written down. An empty list where nothing has been installed and
    /// nobody has typed one, which is a session's `PATH` composed out of the
    /// server's own and nothing else.
    pub fn session_path(&self) -> &[String] {
        &self.session_path
    }

    /// The same config with `session_path` as it already stands.
    ///
    /// What a save from the settings page goes through, that page having no
    /// field for the key: a config built out of what was sent would write the
    /// file with this key gone, and what it named is where a harness Verkstead
    /// installed actually is.
    pub fn keeping_session_path(mut self, kept: &Config) -> Config {
        self.session_path = kept.session_path.clone();

        self
    }

    /// And the same config with `directory` on the end of that list, which is
    /// what an install landing in one writes — see
    /// [`crate::sandbox::installed_into`], the one caller.
    ///
    /// A directory already written down is not written twice: an install run a
    /// second time is the same directory, and a list holding it twice would be
    /// a `PATH` entry searched twice.
    pub fn with_session_path(mut self, directory: &Path) -> Config {
        let directory = directory.to_string_lossy().into_owned();

        if !self.session_path.contains(&directory) {
            self.session_path.push(directory);
        }

        self
    }

    /// And the comments nothing is ever to be dispatched about, in the order
    /// they were written down. An empty list where nobody has added any, which
    /// is every comment on every pull request being somebody's to address.
    pub fn ignored_comments(&self) -> &[IgnoreRule] {
        &self.ignored_comments
    }

    /// And the MCP servers declared for this installation, in the order they
    /// were written down. Empty where nobody has declared any, which is a
    /// Conversation with nothing to attach — and what every installation before
    /// this one looks like.
    pub fn mcp_servers(&self) -> &[McpServer] {
        &self.mcp_servers
    }

    /// And the ones `attached` names, each as its name and the URL it is
    /// reached at — which is what a Conversation's chips come to at the moment
    /// a session is launched.
    ///
    /// **In the order they were attached** rather than the order they were
    /// declared in: the chips are the Conversation's list, and it is the
    /// Conversation being launched.
    ///
    /// **A name nothing declares is left out**, silently as far as the launch
    /// goes. A chip whose declaration has since been deleted is already drawn
    /// as a server that is gone — see the server's
    /// `conversations::attached_servers` — and there is nowhere to send an
    /// agent, so there is nothing to write into a root and nothing to tell a
    /// session about. Holding the launch over it would stop the work for a
    /// reference the human can see is broken.
    pub fn servers_among<'a>(&'a self, attached: &[String]) -> Vec<(&'a str, &'a str)> {
        attached
            .iter()
            .filter_map(|name| {
                self.mcp_servers
                    .iter()
                    .find(|server| server.name() == Some(name.as_str()))
                    .and_then(|server| Some((server.name()?, server.url()?)))
            })
            .collect()
    }

    /// And the same again with the header values `secrets` keeps for each of
    /// them, which is what a Built Root is written out of.
    ///
    /// The one place the two files are put together. [`Config::servers_among`]
    /// answers which servers and where, which is what a prompt names and what a
    /// page could be shown; this answers what is *sent* to them, and a caller
    /// that wanted only the names should ask the other.
    ///
    /// A header the declaration names and the secrets keep nothing for is left
    /// out rather than sent empty: an empty header is not the credential the
    /// service asked for, and a request without one fails where a request with
    /// an empty one fails obscurely — which is [`blank_is_nothing`]'s reason
    /// said about a header.
    pub fn attached_among(&self, attached: &[String], secrets: &Secrets) -> Vec<AttachedServer> {
        self.servers_among(attached)
            .into_iter()
            .map(|(name, url)| AttachedServer {
                name: name.to_owned(),
                url: url.to_owned(),
                headers: self
                    .mcp_servers
                    .iter()
                    .find(|server| server.name() == Some(name))
                    .map(McpServer::headers)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|header| {
                        Some((header.clone(), secrets.mcp_header(name, header)?.to_owned()))
                    })
                    .collect(),
            })
            .collect()
    }

    /// And the text every session is given, which is empty where nobody has
    /// typed one — a session told nothing beyond what its Repo carries.
    ///
    /// A `&str` rather than an `Option`, because the two states a caller could
    /// tell apart are the same state: nothing configured and a text of nothing
    /// are both nothing to say, and every caller of this asks the one question
    /// of whether there is anything to say at all.
    pub fn instructions(&self) -> &str {
        self.instructions.as_deref().unwrap_or_default()
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

/// A [`GitAuthor`] with both halves filled in, which is the only shape git will
/// take one in.
///
/// Git wants an identity rather than half of one, and what it would say about
/// the half that was missing is a sentence about git where the answer is about
/// the settings page that has the field. So everything Verkstead commits on its
/// own account asks for this first and refuses by name where the answer is
/// `None`, rather than filling the missing half in: a created repository's
/// README — see [`crate::repos`] — and the clearing of a task list a branch
/// inherited from its base — see [`crate::tasks::clear`].
///
/// Here beside [`GitAuthor`] rather than in either of them, because it is the
/// same identity under the same rule and one rule wants one place to live.
pub(crate) struct Author {
    name: String,
    email: String,
}

impl Author {
    /// The configured author, or `None` where either half of it is missing.
    pub(crate) fn configured(author: &GitAuthor) -> Option<Author> {
        let (Some(name), Some(email)) = (author.name(), author.email()) else {
            return None;
        };

        Some(Author {
            name: name.to_owned(),
            email: email.to_owned(),
        })
    }

    /// What the commit's `-c user.name=` is given.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// And what its `-c user.email=` is.
    pub(crate) fn email(&self) -> &str {
        &self.email
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

/// One MCP server declared for this installation: a name, and the URL it is
/// reached at.
///
/// **The name is the identity.** It is what a Conversation's chip refers to and
/// what the agent sees in front of its tool names, so it is lowercase letters,
/// digits and hyphens and unique among the declarations — and it is never
/// changed, because everything that refers to a server refers to it by that
/// name. Changing one is deleting the declaration and making another.
///
/// **HTTP and nothing else.** There is no command, no arguments and no choice
/// of transport here: a stdio server is a child process an agent starts inside
/// its own sandbox, which is a hole in the sandbox rather than a setting, and
/// it was turned down in the grilling this was settled in — see ADR-0021.
///
/// Both halves optional for the reason [`IgnoreRule`]'s are: the human
/// hand-edits this file, and a half-written entry is not worth taking the whole
/// of the settings away over. One with either half missing is no declaration at
/// all, and is dropped as the file is read — see [`servers_kept`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct McpServer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,

    /// And the headers it is spoken to with, by name and in the order they are
    /// sent — `Authorization`, an API key's own header, whatever the service's
    /// documentation gives. Empty is a server that wants none, which is the
    /// ordinary declaration.
    ///
    /// **The names only.** Every value is a secret and is in `secrets.yaml`
    /// under this server's name — see [`Secrets::mcp_header`]. What is here is
    /// what may be read back to the page and what says which headers there are
    /// at all; a name declared with nothing kept for it is a header nothing is
    /// sent in.
    ///
    /// Read as leniently as the rows above are, and a blank one is no header:
    /// a row somebody emptied by hand says what an emptied bind says. A name
    /// written twice is one header, because a request has one value per header
    /// and the file has one value per name — see [`McpServer::of`].
    #[serde(
        default,
        deserialize_with = "rows_written",
        skip_serializing_if = "Vec::is_empty"
    )]
    headers: Vec<String>,
}

impl McpServer {
    /// The declaration a settings page has just been told, each blank half
    /// nothing at all — see [`blank_is_nothing`].
    ///
    /// The header names with the blanks taken out and the repeats dropped,
    /// first written first: what is kept for a header is kept under its name,
    /// so two rows of one name are one header whatever the page drew.
    pub fn of(name: Option<String>, url: Option<String>, headers: Vec<String>) -> McpServer {
        let mut kept: Vec<String> = Vec::new();

        for header in entries_written(headers) {
            if !kept.contains(&header) {
                kept.push(header);
            }
        }

        McpServer {
            name: name.and_then(blank_is_nothing),
            url: url.and_then(blank_is_nothing),
            headers: kept,
        }
    }

    /// What a Conversation refers to this server by, and what the agent sees in
    /// front of its tool names.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// And where it is spoken to, which is an HTTP URL.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// And the names of the headers it is spoken to with, in the order they are
    /// sent. Empty is a server that wants none.
    pub fn headers(&self) -> &[String] {
        &self.headers
    }
}

/// What is to become of one header's value on a save: the token's three
/// actions, one per header.
///
/// An action rather than a value for the reason [`crate::settings`]'s token is
/// one, and it is the same reason: the value is write-only, so a page cannot
/// send back what it was never shown, and a blank box read as *clear this*
/// would take a key away every time somebody corrected a URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderValue {
    /// Leave whatever is kept for this header alone, which is what a value box
    /// left blank means.
    Keep,

    /// Send this in it from now on, in place of whatever is kept.
    Set(String),

    /// And take away what is kept, leaving the header declared with nothing to
    /// send in it.
    Clear,
}

/// One attached server as a launch needs it: a name, a URL, and every header it
/// is spoken to with, value and all.
///
/// The two files put together, which is the one place they are — see
/// [`Config::attached_among`], which is the only thing that makes one. A
/// declaration says which headers there are and in what order, and the secrets
/// say what goes in each; what comes out is what is written into a Built Root.
///
/// **The values are here in the clear**, and that is the limit of the secrecy:
/// they are kept from the page and the wire rather than from the agent, which
/// reads its own configuration — see ADR-0021.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedServer {
    name: String,
    url: String,
    headers: Vec<(String, String)>,
}

impl AttachedServer {
    /// One as a caller outside the settings would have it: what a test declares
    /// and attaches in one breath, and what a harness that composes its own
    /// configuration out of these is written against.
    ///
    /// The ordinary way to one is [`Config::attached_among`], which is the two
    /// files put together and the only thing a launch uses.
    pub fn of(name: &str, url: &str, headers: &[(&str, &str)]) -> AttachedServer {
        AttachedServer {
            name: name.to_owned(),
            url: url.to_owned(),
            headers: headers
                .iter()
                .map(|(header, value)| ((*header).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }

    /// What the agent sees in front of this server's tool names.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// And where it is spoken to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// And what is sent with every request to it, in the order the declaration
    /// names them. Empty is a server that wants none, and a header declared
    /// with nothing kept for it is not among them.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }
}

/// What would stop each of `servers` being written down, by where it stands in
/// the list — and empty where there is nothing wrong with any of them.
///
/// Over the whole list rather than one declaration at a time, because one of
/// the two things that can be wrong with a name is that another declaration has
/// it: a server is referred to by name, so two of a name would be a chip
/// pointing at either.
///
/// Every entry at fault rather than the first, for the reason [`IgnoreRule`]'s
/// refusals name every row: the page draws the error at the row, and a human
/// who mistyped two names should be told about both rather than finding the
/// second after fixing the first.
///
/// The first of two names that are the same is not at fault. What is refused is
/// the one that takes a name already spoken for, which is the row the human
/// just typed — and refusing both would leave them correcting a declaration
/// that was there before they arrived.
pub fn trouble_among(servers: &[McpServer]) -> Vec<(usize, ServerTrouble)> {
    servers
        .iter()
        .enumerate()
        .filter_map(|(at, server)| Some((at, trouble(server, &servers[..at])?)))
        .collect()
}

/// What is wrong with one declaration, given the ones written down before it.
fn trouble(server: &McpServer, above: &[McpServer]) -> Option<ServerTrouble> {
    let Some(name) = server.name() else {
        return Some(ServerTrouble::Name(
            "a server is referred to by name, so it needs one".to_owned(),
        ));
    };

    if !named_plainly(name) {
        return Some(ServerTrouble::Name(
            "a name is lowercase letters, digits and hyphens: it is what the agent sees in \
             front of the server's tool names"
                .to_owned(),
        ));
    }

    if above.iter().any(|earlier| earlier.name() == Some(name)) {
        return Some(ServerTrouble::Name(format!(
            "a server is already declared as {name}, and the name is what tells two of them apart"
        )));
    }

    if server.url().is_none() {
        return Some(ServerTrouble::Url(
            "a server is reached over HTTP, so it needs a URL".to_owned(),
        ));
    }

    None
}

/// Whether a name is the lowercase letters, digits and hyphens a server's is —
/// and something rather than nothing, an empty name having been read as no name
/// at all long before this.
///
/// ASCII throughout rather than Unicode's own idea of a lowercase letter: what
/// the name is for is a tool name an agent reads and a human types on a phone,
/// and two names that differ by a character nobody can see would be two servers
/// nobody can tell apart.
fn named_plainly(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|letter| letter.is_ascii_lowercase() || letter.is_ascii_digit() || letter == '-')
}

/// What is wrong with a server somebody tried to declare.
///
/// Which of the two fields, always: the page draws the error at the box it is
/// about, and every way a declaration goes wrong is a way one of its two halves
/// does. Which is what tells this from [`RuleTrouble`], where a rule giving
/// neither field is wrong as a whole and has no box to be drawn at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerTrouble {
    /// The name is missing, is not lowercase letters, digits and hyphens, or is
    /// one another declaration already has — in words to put on the row.
    Name(String),

    /// And the URL is missing.
    Url(String),
}

/// A list of declarations as somebody left them, with the rows they emptied out
/// taken away.
///
/// Written for the reason [`rows_written`] is: a row with nothing after its
/// `-` is YAML's null, and a `Vec<McpServer>` reading one would refuse the whole
/// file — which under this module's own rule would throw the author and the
/// build cache away over a half-deleted line.
fn servers_written<'de, D: serde::Deserializer<'de>>(
    servers: D,
) -> Result<Vec<McpServer>, D::Error> {
    Ok(Vec::<Option<McpServer>>::deserialize(servers)?
        .into_iter()
        .flatten()
        .collect())
}

/// A written list of declarations with the blanks taken out of each and the
/// ones that came to nothing dropped.
///
/// A declaration wants both halves to be one at all: a name is what everything
/// refers to it by, and a URL is where it is. So an entry missing either is
/// dropped as the file is read, the way a rule constraining nothing is — and it
/// fails in the safe direction, a server nobody can reach simply not being
/// there to attach.
///
/// A name this would not have been given by the settings page — one with a
/// capital in it, or one a second entry repeats — is kept exactly as it was
/// hand-edited. The refusing is the save's, and a read that dropped it would be
/// a declaration the human could neither use nor see to correct.
fn servers_kept(servers: Vec<McpServer>) -> Vec<McpServer> {
    servers
        .into_iter()
        .map(|server| McpServer::of(server.name, server.url, server.headers))
        .filter(|server| server.name.is_some() && server.url.is_some())
        .collect()
}

/// The header values in `secrets.yaml` with the blanks taken out of them: a
/// value somebody emptied by hand is a header nothing is sent in, and a server
/// left holding none is dropped.
///
/// Which is exactly what [`Secrets::with_mcp_headers`] writes, said again for
/// the file it reads: what a hand-edit leaves behind and what a save leaves
/// behind have to mean the same thing, or a header would be sent empty on a
/// machine somebody had opened the file on.
///
/// The names are not touched. A header the declaration does not name is never
/// looked up, so one left here by an edit to `config.yaml` is spent rather than
/// wrong — and dropping it would be throwing away a value the human could not
/// type again.
fn headers_kept(
    headers: BTreeMap<String, BTreeMap<String, String>>,
) -> BTreeMap<String, BTreeMap<String, String>> {
    headers
        .into_iter()
        .map(|(server, sent)| {
            let sent: BTreeMap<String, String> = sent
                .into_iter()
                .filter_map(|(header, value)| Some((header, blank_is_nothing(value)?)))
                .collect();

            (server, sent)
        })
        .filter(|(server, sent)| !server.trim().is_empty() && !sent.is_empty())
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

/// And a configured *text* that is only whitespace is no text — with everything
/// else kept exactly as it was typed.
///
/// The one value in either file read this way. [`blank_is_nothing`] trims,
/// because what it reads are names, addresses and paths, and a stray space
/// around one of those is a typo. What this reads is the human's own prose for
/// an agent, where the leading spaces of an indented list and the blank line
/// that ends a paragraph are the writing rather than slips in it — and a
/// harness is handed this verbatim, so a save that tidied it would be one
/// nobody asked for.
fn prose_written(text: String) -> Option<String> {
    (!text.trim().is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        AttachedServer, Author, Cleanup, CleanupStep, Config, ConflictResolution, GitAuthor,
        HeaderValue, IgnoreRule, McpServer, RuleTrouble, RustBuildCache, Secrets, ServerTrouble,
        Settings, trouble_among,
    };

    #[test]
    fn the_token_is_what_the_file_says() {
        let secrets = Secrets::read("github_token: ghp_thetoken\n").unwrap();

        assert_eq!(secrets.github_token(), Some("ghp_thetoken"));
    }

    #[test]
    fn the_session_accounts_password_is_read_beside_the_token() {
        let secrets =
            Secrets::read("github_token: ghp_thetoken\nsession_account_password: Vk1-hunter2\n")
                .unwrap();

        assert_eq!(secrets.github_token(), Some("ghp_thetoken"));
        assert_eq!(secrets.session_account_password(), Some("Vk1-hunter2"));
    }

    #[test]
    fn a_blank_password_is_no_password() {
        assert_eq!(
            Secrets::read("session_account_password: ''\n")
                .unwrap()
                .session_account_password(),
            None
        );
    }

    #[test]
    fn a_stored_password_survives_the_token_being_cleared() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        // The elevated verb's write, and then the settings page's — in that
        // order, because that is the order a machine does them in.
        settings
            .save_secrets(
                &settings
                    .secrets()
                    .with_session_account_password(Some("Vk1-hunter2".to_owned())),
            )
            .unwrap();
        settings
            .save_secrets(
                &settings
                    .secrets()
                    .with_token(Some("ghp_thetoken".to_owned())),
            )
            .unwrap();

        // And now somebody tidies up the token they had finished with, which is
        // what used to empty this file.
        settings
            .save_secrets(&settings.secrets().with_token(None))
            .unwrap();

        assert_eq!(settings.secrets().github_token(), None);
        assert_eq!(
            settings.secrets().session_account_password(),
            Some("Vk1-hunter2"),
            "clearing a token should not have taken the session account's password with it",
        );
    }

    /// The header values are read under the server's name and then the
    /// header's, which is the shape that lets a declaration be deleted by
    /// leaving its name out of the next save.
    #[test]
    fn the_header_values_are_read_under_the_server_that_declared_them() {
        let secrets = Secrets::read(concat!(
            "github_token: ghp_thetoken\n",
            "mcp_headers:\n",
            "  docs:\n",
            "    Authorization: Bearer sk-averysecretkey\n",
            "    X-Tenant: verkstead\n",
        ))
        .unwrap();

        assert_eq!(
            secrets.mcp_header("docs", "Authorization"),
            Some("Bearer sk-averysecretkey")
        );
        assert_eq!(secrets.mcp_header("docs", "X-Tenant"), Some("verkstead"));
        assert_eq!(secrets.mcp_header("docs", "X-Nothing"), None);
        assert_eq!(secrets.mcp_header("tickets", "Authorization"), None);
        assert_eq!(secrets.github_token(), Some("ghp_thetoken"));
    }

    /// And a value somebody emptied by hand is a header nothing is sent in,
    /// which is what a save writes when one is cleared.
    #[test]
    fn a_blank_header_value_is_no_header() {
        let secrets = Secrets::read(concat!(
            "mcp_headers:\n",
            "  docs:\n",
            "    Authorization: ''\n",
            "  tickets:\n",
            "    X-Tenant: verkstead\n",
        ))
        .unwrap();

        assert_eq!(secrets.mcp_header("docs", "Authorization"), None);
        assert_eq!(secrets.mcp_header("tickets", "X-Tenant"), Some("verkstead"));
    }

    /// Each header is kept, set or cleared on its own, and a server left out of
    /// the save loses what was kept for it — which is how deleting a
    /// declaration takes its secrets with it.
    #[test]
    fn a_header_is_kept_set_or_cleared_and_a_server_left_out_loses_its_own() {
        let kept = Secrets::read(concat!(
            "mcp_headers:\n",
            "  docs:\n",
            "    Authorization: Bearer sk-thefirstkey\n",
            "    X-Tenant: verkstead\n",
            "  tickets:\n",
            "    Authorization: Bearer sk-thetickets\n",
        ))
        .unwrap();

        let written = kept.with_mcp_headers(&[(
            "docs".to_owned(),
            vec![
                (
                    "Authorization".to_owned(),
                    HeaderValue::Set("Bearer sk-thesecondkey".to_owned()),
                ),
                ("X-Tenant".to_owned(), HeaderValue::Keep),
                ("X-Spent".to_owned(), HeaderValue::Clear),
            ],
        )]);

        assert_eq!(
            written.mcp_header("docs", "Authorization"),
            Some("Bearer sk-thesecondkey")
        );
        assert_eq!(written.mcp_header("docs", "X-Tenant"), Some("verkstead"));
        assert_eq!(written.mcp_header("docs", "X-Spent"), None);
        assert_eq!(
            written.mcp_header("tickets", "Authorization"),
            None,
            "a server the save did not send is one nobody declares any more"
        );
    }

    /// And the token beside them is neither read nor written by that: the file
    /// is written whole, and the two hands that write it may not take each
    /// other's work away.
    #[test]
    fn writing_the_headers_leaves_the_token_and_the_password_where_they_are() {
        let kept = Secrets::read(concat!(
            "github_token: ghp_thetoken\n",
            "session_account_password: Vk1-hunter2\n",
        ))
        .unwrap();

        let written = kept.with_mcp_headers(&[(
            "docs".to_owned(),
            vec![(
                "Authorization".to_owned(),
                HeaderValue::Set("Bearer sk-averysecretkey".to_owned()),
            )],
        )]);

        assert_eq!(written.github_token(), Some("ghp_thetoken"));
        assert_eq!(written.session_account_password(), Some("Vk1-hunter2"));

        // And the other way about.
        let cleared = written.with_token(None);

        assert_eq!(
            cleared.mcp_header("docs", "Authorization"),
            Some("Bearer sk-averysecretkey")
        );
    }

    /// A file holding nothing but headers that have all been cleared says what
    /// an unwritten one says, which is what [`Settings::save_secrets`] empties
    /// it over.
    #[test]
    fn headers_cleared_to_nothing_leave_nothing_configured() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(&settings.secrets().with_mcp_headers(&[(
                "docs".to_owned(),
                vec![(
                    "Authorization".to_owned(),
                    HeaderValue::Set("Bearer sk-averysecretkey".to_owned()),
                )],
            )]))
            .unwrap();

        assert_eq!(
            settings.secrets().mcp_header("docs", "Authorization"),
            Some("Bearer sk-averysecretkey")
        );

        settings
            .save_secrets(&settings.secrets().with_mcp_headers(&[(
                "docs".to_owned(),
                vec![("Authorization".to_owned(), HeaderValue::Clear)],
            )]))
            .unwrap();

        assert!(
            std::fs::read_to_string(settings.secrets_path())
                .unwrap()
                .is_empty()
        );
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
                String::new(),
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
                String::new(),
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
                String::new(),
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
                String::new(),
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
                String::new(),
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
                String::new(),
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
            .save_secrets(
                &settings
                    .secrets()
                    .with_token(Some("ghp_thetoken".to_owned())),
            )
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
            .save_secrets(
                &settings
                    .secrets()
                    .with_token(Some("ghp_thetoken".to_owned())),
            )
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
            .save_secrets(
                &settings
                    .secrets()
                    .with_token(Some("ghp_thetoken".to_owned())),
            )
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
            .save_secrets(
                &settings
                    .secrets()
                    .with_token(Some("ghp_thetoken".to_owned())),
            )
            .unwrap();
        settings
            .save_secrets(&settings.secrets().with_token(None))
            .unwrap();

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
            .save_secrets(&settings.secrets().with_token(Some("   \n".to_owned())))
            .unwrap();

        assert_eq!(settings.secrets().github_token(), None);
    }

    #[test]
    fn a_pasted_token_keeps_none_of_the_whitespace_that_came_with_it() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_secrets(
                &settings
                    .secrets()
                    .with_token(Some(" ghp_thetoken\n".to_owned())),
            )
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
                String::new(),
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
                String::new(),
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
                String::new(),
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
                String::new(),
            ))
            .unwrap();

        let config = settings.config();

        assert_eq!(config.git_author().name(), Some("Tobias Cohen"));
        assert_eq!(config.git_author().email(), None);
    }

    /// And the shape git will take is both halves or nobody — the reading
    /// everything Verkstead commits on its own account asks for.
    #[test]
    fn an_author_is_both_halves_or_nobody() {
        let name = Some("Ada Lovelace".to_owned());
        let email = Some("ada@example.com".to_owned());

        let both = Author::configured(&GitAuthor::of(name.clone(), email.clone()))
            .expect("both halves are there");

        assert_eq!(both.name(), "Ada Lovelace");
        assert_eq!(both.email(), "ada@example.com");

        assert!(Author::configured(&GitAuthor::of(name, None)).is_none());
        assert!(Author::configured(&GitAuthor::of(None, email)).is_none());
        assert!(Author::configured(&GitAuthor::default()).is_none());
    }

    #[test]
    fn there_is_no_written_time_until_something_has_been_written() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        assert!(settings.secrets_written_at().is_none());

        settings
            .save_secrets(
                &settings
                    .secrets()
                    .with_token(Some("ghp_thetoken".to_owned())),
            )
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
            .save_secrets(
                &settings
                    .secrets()
                    .with_token(Some("ghp_thetoken".to_owned())),
            )
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
                String::new(),
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
            .save_secrets(
                &settings
                    .secrets()
                    .with_token(Some("ghp_thetoken".to_owned())),
            )
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
                String::new(),
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
                String::new(),
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
                String::new(),
            ))
            .unwrap();

        assert!(settings.config().sandbox_binds().is_empty());
    }

    /// `session_path` is read the way every other list here is: in the order it
    /// was written, blanks dropped and the rest trimmed.
    #[test]
    fn the_session_path_is_what_the_config_file_says_in_the_order_it_says_it() {
        let config = Config::read(
            "session_path:\n  - /home/you/.local/bin\n  - ''\n  \
             - '  /home/you/.grok/bin  '\n  -\n",
        )
        .unwrap();

        assert_eq!(
            config.session_path(),
            ["/home/you/.local/bin", "/home/you/.grok/bin"],
        );
    }

    /// And a key nothing can make a list of says what an absent one says, which
    /// is what every other key in this file says of a value it cannot read.
    #[test]
    fn a_file_with_no_session_path_in_it_or_a_malformed_one_configures_none() {
        assert!(
            Config::read("git_author:\n  name: Ada\n")
                .unwrap()
                .session_path()
                .is_empty()
        );
        assert!(Config::read("").unwrap().session_path().is_empty());

        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());
        std::fs::write(settings.config_path(), "session_path: 5\n").unwrap();

        assert!(
            settings.config().session_path().is_empty(),
            "a file Verkstead cannot read is nothing configured, logged — and \
             not a `PATH` half-composed out of what parsed",
        );
    }

    /// A directory an install landed in goes on the end of the list, and a
    /// second install into the same one does not write it twice.
    #[test]
    fn an_install_appends_the_directory_it_landed_in_and_never_twice() {
        let config = Config::default()
            .with_session_path(Path::new("/home/you/.local/bin"))
            .with_session_path(Path::new("/home/you/.grok/bin"))
            .with_session_path(Path::new("/home/you/.local/bin"));

        assert_eq!(
            config.session_path(),
            ["/home/you/.local/bin", "/home/you/.grok/bin"],
            "a `PATH` entry written twice is one searched twice, and the second \
             install is the same directory as the first",
        );
    }

    /// And a save from the settings page keeps it: that page has no field for
    /// the key, so a config built out of what it sent would take away the
    /// directory a harness Verkstead installed is really in.
    #[test]
    fn a_save_from_the_settings_page_keeps_the_session_path_it_was_not_told_about() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&Config::default().with_session_path(Path::new("/home/you/.local/bin")))
            .unwrap();

        let page = Config::of(
            GitAuthor::of(Some("Ada".to_owned()), Some("ada@example.com".to_owned())),
            RustBuildCache::default(),
            Cleanup::default(),
            ConflictResolution::Merge,
            false,
            vec![],
            vec![],
            vec![],
            String::new(),
        );

        settings
            .save_config(&page.keeping_session_path(&settings.config()))
            .unwrap();

        assert_eq!(
            settings.config().session_path(),
            ["/home/you/.local/bin"],
            "the page said nothing about it, so the save said nothing about it \
             either",
        );
        assert_eq!(
            settings.config().git_author().name(),
            Some("Ada"),
            "and what the page did send is what the file holds",
        );
    }

    /// The text every session is given is what the file says, and the three
    /// ways of having said nothing all say nothing.
    #[test]
    fn the_instructions_are_what_the_config_file_says() {
        assert_eq!(
            Config::read("instructions: Prefer the smallest change.\n")
                .unwrap()
                .instructions(),
            "Prefer the smallest change.",
        );

        assert_eq!(
            Config::read("git_author:\n  name: Ada\n")
                .unwrap()
                .instructions(),
            "",
            "a file with no such key is a session told nothing",
        );

        assert_eq!(
            Config::read("instructions: '   '\n")
                .unwrap()
                .instructions(),
            "",
            "and so is a text that is nothing but whitespace",
        );

        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        assert_eq!(
            settings.config().instructions(),
            "",
            "and so is a Data Directory with no config file in it at all",
        );

        std::fs::write(settings.config_path(), "instructions: [oh\n").unwrap();

        assert_eq!(
            settings.config().instructions(),
            "",
            "and so is a file nothing can parse",
        );
    }

    /// And a text that is nothing but whitespace is nothing at all, including in
    /// the file a save leaves behind: an empty key would read as a setting
    /// somebody made.
    #[test]
    fn instructions_cleared_are_written_away_rather_than_written_empty() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        settings
            .save_config(&config_saying("Prefer the smallest change."))
            .unwrap();

        assert!(
            std::fs::read_to_string(settings.config_path())
                .unwrap()
                .contains("instructions"),
            "the text somebody typed is in the file",
        );

        settings.save_config(&config_saying("")).unwrap();

        let written = std::fs::read_to_string(settings.config_path()).unwrap();

        assert!(
            !written.contains("instructions"),
            "and clearing the box takes the key out rather than leaving an \
             empty one: {written}",
        );
        assert_eq!(settings.config().instructions(), "");
    }

    /// And what goes through the file comes back as it was typed — the blank
    /// line between two paragraphs, the indent of a list, and all.
    ///
    /// This is the whole of what the setting is for: a harness is handed these
    /// words, so a save that reflowed them would be handing it something nobody
    /// wrote.
    #[test]
    fn the_instructions_go_through_the_file_verbatim() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        let text = "Prefer the smallest change that does the job.\n\nAnd:\n  - run the tests\n  - say what broke\n";

        settings.save_config(&config_saying(text)).unwrap();

        assert_eq!(settings.config().instructions(), text);
    }

    /// A config holding nothing but a text, which is what the three tests above
    /// save.
    fn config_saying(instructions: &str) -> Config {
        Config::of(
            GitAuthor::default(),
            RustBuildCache::default(),
            Cleanup::default(),
            ConflictResolution::Merge,
            false,
            vec![],
            vec![],
            vec![],
            instructions.to_owned(),
        )
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
                vec![IgnoreRule::of(
                    Some("coderabbitai".to_owned()),
                    Some("billing".to_owned()),
                )],
                vec![],
                String::new(),
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

    #[test]
    fn the_declared_servers_are_what_the_config_file_says() {
        let config = Config::read(
            "mcp_servers:\n  - name: docs\n    url: https://mcp.example.com/docs\n  - name: tickets\n    url: https://mcp.example.com/tickets\n",
        )
        .unwrap();

        let servers = config.mcp_servers();

        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].name(), Some("docs"));
        assert_eq!(servers[0].url(), Some("https://mcp.example.com/docs"));
        assert_eq!(servers[1].name(), Some("tickets"));
        assert_eq!(servers[1].url(), Some("https://mcp.example.com/tickets"));
    }

    /// The three ways of saying nothing, which all say the same thing: no
    /// servers, and no failure to report.
    #[test]
    fn a_file_with_no_declared_servers_says_none() {
        assert!(
            Config::read("git_author:\n  name: Ada\n")
                .unwrap()
                .mcp_servers()
                .is_empty()
        );
        assert!(Config::read("").unwrap().mcp_servers().is_empty());
        assert!(
            Config::read("mcp_servers:\n")
                .unwrap()
                .mcp_servers()
                .is_empty()
        );
    }

    /// And a key nothing can parse is no servers as well, rather than a read
    /// that fails: what [`Settings::config`] does with a file it cannot read is
    /// log it and configure nothing, which is this module's rule about
    /// everything it is told.
    #[test]
    fn a_servers_key_nothing_can_parse_reads_as_no_servers() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::in_data_dir(dir.path());

        std::fs::write(settings.config_path(), "mcp_servers: what\n").unwrap();

        assert!(settings.config().mcp_servers().is_empty());
    }

    /// A row somebody half-deleted, and a declaration missing either half: all
    /// of them nothing rather than a file that will not read.
    #[test]
    fn half_a_declaration_is_no_declaration() {
        let config = Config::read(
            "mcp_servers:\n  -\n  - {}\n  - name: docs\n  - url: https://mcp.example.com/docs\n  - name: '  '\n    url: https://mcp.example.com/docs\n  - name: tickets\n    url: https://mcp.example.com/tickets\n",
        )
        .unwrap();

        let servers = config.mcp_servers();

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name(), Some("tickets"));
    }

    /// A name the settings page would have refused is kept exactly as it was
    /// hand-edited, the way a pattern that will not compile is: the human has to
    /// be able to see it on the page to correct it.
    #[test]
    fn a_hand_edited_name_the_page_would_refuse_is_kept() {
        let config =
            Config::read("mcp_servers:\n  - name: Docs Server\n    url: https://example.com\n")
                .unwrap();

        assert_eq!(config.mcp_servers()[0].name(), Some("Docs Server"));
    }

    #[test]
    fn a_saved_declaration_is_what_the_next_read_says() {
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
                vec![McpServer::of(
                    Some("docs".to_owned()),
                    Some("https://mcp.example.com/docs".to_owned()),
                    vec![],
                )],
                String::new(),
            ))
            .unwrap();

        let config = settings.config();
        let servers = config.mcp_servers();

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name(), Some("docs"));
        assert_eq!(servers[0].url(), Some("https://mcp.example.com/docs"));
    }

    /// What the settings page is refused over: a name that is not a name, a name
    /// somebody else has, and a server with nowhere to be reached.
    #[test]
    fn a_declaration_says_what_is_wrong_with_it() {
        assert_eq!(
            trouble_among(&[declared("docs", "https://example.com")]),
            []
        );

        assert!(matches!(
            trouble_among(&[declared("Docs", "https://example.com")]).as_slice(),
            [(0, ServerTrouble::Name(_))]
        ));
        assert!(matches!(
            trouble_among(&[declared("docs server", "https://example.com")]).as_slice(),
            [(0, ServerTrouble::Name(_))]
        ));
        assert!(matches!(
            trouble_among(&[declared("docs_server", "https://example.com")]).as_slice(),
            [(0, ServerTrouble::Name(_))]
        ));
        assert!(matches!(
            trouble_among(&[McpServer::of(
                None,
                Some("https://example.com".to_owned()),
                vec![]
            )])
            .as_slice(),
            [(0, ServerTrouble::Name(_))]
        ));
        assert!(matches!(
            trouble_among(&[McpServer::of(Some("docs".to_owned()), None, vec![])]).as_slice(),
            [(0, ServerTrouble::Url(_))]
        ));

        // A hyphen and a digit are a name, which is what the rule allows and
        // what every declaration anybody actually writes looks like.
        assert_eq!(
            trouble_among(&[declared("docs-2", "https://example.com")]),
            []
        );
    }

    /// The second of two of a name is the one at fault: it is the row the human
    /// just typed, and refusing both would send them to correct a declaration
    /// that was there before they arrived.
    #[test]
    fn the_name_that_takes_one_already_declared_is_the_one_refused() {
        let trouble = trouble_among(&[
            declared("docs", "https://example.com/one"),
            declared("docs", "https://example.com/two"),
        ]);

        assert!(matches!(trouble.as_slice(), [(1, ServerTrouble::Name(_))]));
    }

    /// Every row at fault rather than the first, because the page draws the
    /// error at the row.
    #[test]
    fn every_declaration_at_fault_is_named() {
        let trouble = trouble_among(&[
            declared("Docs", "https://example.com"),
            declared("tickets", "https://example.com"),
            McpServer::of(Some("notes".to_owned()), None, vec![]),
        ]);

        assert!(matches!(
            trouble.as_slice(),
            [(0, ServerTrouble::Name(_)), (2, ServerTrouble::Url(_))]
        ));
    }

    /// One declaration, for the tests above.
    fn declared(name: &str, url: &str) -> McpServer {
        McpServer::of(Some(name.to_owned()), Some(url.to_owned()), vec![])
    }

    /// What a Conversation's chips come to at a launch: the declarations they
    /// name, in the order the human attached them rather than the order they
    /// were declared in.
    #[test]
    fn the_servers_a_conversation_attached_come_back_in_its_own_order() {
        let config = Config::read(concat!(
            "mcp_servers:\n",
            "  - name: docs\n    url: https://mcp.example.com/docs\n",
            "  - name: tickets\n    url: https://mcp.example.com/tickets\n",
            "  - name: alerts\n    url: https://mcp.example.com/alerts\n",
        ))
        .unwrap();

        assert_eq!(
            config.servers_among(&["tickets".to_owned(), "docs".to_owned()]),
            vec![
                ("tickets", "https://mcp.example.com/tickets"),
                ("docs", "https://mcp.example.com/docs"),
            ],
        );
        assert!(
            config.servers_among(&[]).is_empty(),
            "and a Conversation with nothing attached has nothing to launch with"
        );
    }

    /// And what a launch is actually handed: the declaration's headers in the
    /// order it names them, each with the value the secrets keep for it — the
    /// one place the two files are put together.
    #[test]
    fn an_attached_server_carries_the_headers_the_secrets_keep_for_it() {
        let config = Config::read(concat!(
            "mcp_servers:\n",
            "  - name: docs\n",
            "    url: https://mcp.example.com/docs\n",
            "    headers:\n",
            "      - Authorization\n",
            "      - X-Tenant\n",
            "      - X-Nothing-Kept\n",
            "  - name: tickets\n    url: https://mcp.example.com/tickets\n",
        ))
        .unwrap();

        let secrets = Secrets::read(concat!(
            "mcp_headers:\n",
            "  docs:\n",
            "    Authorization: Bearer sk-averysecretkey\n",
            "    X-Tenant: verkstead\n",
            // A value kept under a name no declaration carries is spent rather
            // than sent: what says which headers there are is `config.yaml`.
            "    X-Forgotten: from-an-older-declaration\n",
            "  tickets:\n",
            "    Authorization: Bearer sk-thetickets\n",
        ))
        .unwrap();

        let attached = config.attached_among(&["docs".to_owned(), "tickets".to_owned()], &secrets);

        assert_eq!(
            attached,
            vec![
                AttachedServer::of(
                    "docs",
                    "https://mcp.example.com/docs",
                    &[
                        ("Authorization", "Bearer sk-averysecretkey"),
                        ("X-Tenant", "verkstead"),
                    ],
                ),
                // Declared with no headers at all, so nothing kept under its
                // name is sent.
                AttachedServer::of("tickets", "https://mcp.example.com/tickets", &[]),
            ],
        );
    }

    /// A name nothing declares any more is left out, and what is beside it
    /// still launches: the chip says the server is gone, and the work does not
    /// wait on the human going and fixing it.
    #[test]
    fn a_name_nothing_declares_is_left_out() {
        let config =
            Config::read("mcp_servers:\n  - name: docs\n    url: https://mcp.example.com/docs\n")
                .unwrap();

        assert_eq!(
            config.servers_among(&["deleted".to_owned(), "docs".to_owned()]),
            vec![("docs", "https://mcp.example.com/docs")],
        );
        assert!(
            Config::read("")
                .unwrap()
                .servers_among(&["docs".to_owned()])
                .is_empty(),
            "and an installation that declares none declares none"
        );
    }
}
