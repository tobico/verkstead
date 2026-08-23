//! The settings files: what Verkstead is told, rather than what it finds.
//!
//! GitHub auth used to be whatever happened to sit in the service's home — the
//! host's `~/.config/gh`, bound into every sandbox and hoped to be logged in.
//! That is credentials by accident: nobody says which account a session runs
//! as, nothing says whether one is configured at all, and the failure arrives
//! inside a sandbox as `gh` claiming it has never heard of the machine.
//!
//! So the credentials are said instead, in files of Verkstead's own under the
//! Data Directory beside the database. `secrets.yaml` is the one with anything
//! secret in it:
//!
//! ```yaml
//! github_token: ghp_...
//! ```
//!
//! Read at the moment it is needed rather than held from startup, so a token
//! saved or rotated through the settings page applies to the next session
//! without a restart — and the running sessions keep the environment they
//! started with, which is what they would keep anyway.
//!
//! **Nothing here is ever an error.** A file that is not there, one that is
//! empty, and one nothing can parse all come back as no token configured: the
//! consequence of no token is `gh` inside saying it is not logged in, and the
//! consequence of refusing would be a session that never starts. The malformed
//! case is logged, because a file the human wrote and Verkstead cannot read is
//! the one of the three they would want telling about.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// What the secrets file is called inside the Data Directory. Fixed rather than
/// configurable, for the reason the database's name is: the directory is what an
/// operator points Verkstead at, and what is in it is Verkstead's to name.
const SECRETS: &str = "secrets.yaml";

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

    /// What `secrets.yaml` holds now.
    ///
    /// Blocking, and called where blocking is allowed: a session's sandbox is
    /// built on a blocking thread already, because git is asked about the
    /// worktree there.
    pub fn secrets(&self) -> Secrets {
        let path = self.secrets_path();

        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(path = %path.display(), "no secrets file, so no token");
                return Secrets::default();
            }
            Err(error) => {
                tracing::warn!(
                    error = ?error,
                    path = %path.display(),
                    "the secrets file could not be read, so nothing is configured from it"
                );
                return Secrets::default();
            }
        };

        Secrets::read(&text).unwrap_or_else(|error| {
            tracing::warn!(
                error = %error,
                path = %path.display(),
                "the secrets file is not YAML this understands, so nothing is configured from it"
            );
            Secrets::default()
        })
    }
}

/// What `secrets.yaml` says. Flat, because there is one secret in it.
///
/// Unknown keys are ignored rather than refused: the human hand-edits this file,
/// and a key from a later Verkstead — or a comment they left as a key by mistake
/// — is not worth taking a session's credentials away over.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Secrets {
    /// The GitHub token every session and every host-side `gh` authenticates
    /// with, or `None` where none is configured.
    #[serde(default)]
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

    /// The configured GitHub token, or `None` where there is none.
    pub fn github_token(&self) -> Option<&str> {
        self.github_token.as_deref()
    }
}

/// A configured value that is only whitespace is no value: a token field left
/// empty by hand reads as the human having cleared it, and `GH_TOKEN=` set to
/// nothing at all is a session that fails obscurely rather than one that says it
/// has no login.
fn blank_is_nothing(value: String) -> Option<String> {
    let trimmed = value.trim();

    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{Secrets, Settings};

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
}
