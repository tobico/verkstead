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

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// What the secrets file is called inside the Data Directory. Fixed rather than
/// configurable, for the reason the database's name is: the directory is what an
/// operator points Verkstead at, and what is in it is Verkstead's to name.
const SECRETS: &str = "secrets.yaml";

/// And what the other one is called: everything configured that is nobody's
/// secret.
const CONFIG: &str = "config.yaml";

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

/// What `config.yaml` says: everything told to Verkstead that is nobody's
/// secret.
///
/// Unknown keys are ignored for the reason [`Secrets`]'s are.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    /// Who a session commits as.
    #[serde(default)]
    git_author: GitAuthor,
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
        })
    }

    /// Who a session commits as, which may be nobody.
    pub fn git_author(&self) -> &GitAuthor {
        &self.git_author
    }
}

/// The name and the email address a session's commits are by.
///
/// Two halves, each on its own: a human who has filled in one and not the other
/// gets the one they filled in, and git says what is still missing. Nothing here
/// substitutes a default — a commit by `verkstead@localhost` is worse than a
/// commit that would not be made, because it is the one nobody notices.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GitAuthor {
    #[serde(default)]
    name: Option<String>,

    #[serde(default)]
    email: Option<String>,
}

impl GitAuthor {
    /// What `user.name` is inside a sandbox, where one is configured.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// And what `user.email` is.
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
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
    use super::{Config, Secrets, Settings};

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
}
