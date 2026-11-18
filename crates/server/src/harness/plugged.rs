//! A harness a plugin declares: the sixth implementation of [`Harness`], and
//! the only one a third-party harness is ever (ADR-0030, *Agent*).
//!
//! **Built from what the plugin's manifest says and what its component
//! exports.** Everything about the harness that is data is read off its
//! `agent` provision — see [`crate::plugins::manifest::agent`] — and answered
//! here as the built-ins answer it out of their own modules. The three things
//! that are code are the component's exports, and this calls them with what the
//! host holds: so far `write-configuration`, which is what a root's
//! configuration is.
//!
//! **What the host still does is the host's.** The account's own MCP servers
//! are taken out of its configuration before the export sees any of it (the
//! strip-and-never-write-back rule of ADR-0011) — see [`serverless`] — and
//! the mounting, the joining and the nudging are the same code every built-in
//! goes through.
//!
//! **The account is said from its first path**, which the manifest reader
//! holds to being a directory: that is where it lands in HOME, where its login
//! and its memory are said from, and what the root is built in place of. The
//! paths after it are judged on a save like the first, and are not yet given
//! to a session.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use verkstead_render::{Broken, ProfileSaved};

use super::{
    Carrying, Declared, HOME_REFUSED, Harness, Line, ReadIn, Resume, SETTINGS_FILE, SignedOut,
};
use crate::platform::Platform;
use crate::plugins::component::Loaded;
use crate::plugins::component::worlds::category::agent as world;
use crate::plugins::manifest::{self, Agent};
use crate::sandbox::Home;
use crate::sandbox::root::{Part, Place};
use crate::settings::AttachedServer;
use crate::store;
use crate::transcript::Search;

/// What a file of an account is refused as on a save — see
/// [`Declared::refused`].
const FILE_REFUSED: [ProfileSaved; 3] = [
    ProfileSaved::ConfigNotAbsolute,
    ProfileSaved::ConfigMissing,
    ProfileSaved::NotAFile,
];

/// How long one call into a plugin's component may take before it is stopped.
const CALLING: Duration = Duration::from_secs(10);

/// A harness declared by a plugin: its id, the `agent` its manifest provides,
/// its mark, and its component standing.
pub(crate) struct Plugged {
    id: store::AgentType,
    agent: Agent,
    mark: String,
    component: Component,
}

impl Plugged {
    /// The harness `agent` declares, under the plugin `id`, drawn with `mark`
    /// and run by the component `loaded`.
    pub(crate) fn new(id: store::AgentType, agent: Agent, mark: String, loaded: Loaded) -> Self {
        Plugged {
            id,
            agent,
            mark,
            component: Component::standing(loaded),
        }
    }

    /// The `agent` it was declared as, which is what says whether a plugin
    /// read again is the same harness.
    pub(crate) fn declared(&self) -> &Agent {
        &self.agent
    }

    /// The first of the account's paths, which the account is said from.
    fn first(&self) -> &manifest::agent::AccountPath {
        &self.agent.account[0]
    }

    /// Of the first path's allowlist, what is neither its login nor its
    /// memory: the account's configuration, handed to `write-configuration`.
    fn configured_files(&self) -> Vec<PathBuf> {
        let first = self.first();

        first
            .allowlist
            .iter()
            .filter(|path| !first.credentials.contains(path) && !first.memory.contains(path))
            .cloned()
            .collect()
    }
}

impl Harness for Plugged {
    fn binary(&self) -> &str {
        &self.agent.binary
    }

    fn named(&self) -> &str {
        &self.agent.name
    }

    fn mark(&self) -> &str {
        &self.mark
    }

    fn paths(&self) -> Vec<Declared<'_>> {
        self.agent
            .account
            .iter()
            .map(|path| Declared {
                key: &path.key,
                label: &path.label,
                purpose: path.purpose.as_deref().unwrap_or_default(),
                placeholder: path.example.as_deref().unwrap_or_default(),
                file: path.file,
                // A path a plugin declares is judged, never made: only a
                // built-in whose account its own login makes says otherwise.
                made: false,
                refused: if path.file {
                    FILE_REFUSED
                } else {
                    HOME_REFUSED
                },
                within: &[],
            })
            .collect()
    }

    fn found_at(&self) -> Vec<&str> {
        self.agent
            .account
            .iter()
            .filter_map(|path| path.mounted.to_str())
            .collect()
    }

    /// The declared line as it is written, placeholders and all: the host
    /// fills them as the line is built — see [`super::filled`].
    fn line(&self, _worktree: Option<&Path>, _companions: &[PathBuf]) -> Line {
        let launch = &self.agent.launch;

        Line {
            model: launch.model.clone(),
            prompt: launch.prompt.clone(),
            session: launch.session.clone(),
            resume: launch.resume.as_ref().map(|resume| match resume {
                manifest::agent::Resume::Flag(words) => Resume::Flag(words.clone()),
                manifest::agent::Resume::Subcommand { words, also } => Resume::Subcommand {
                    words: words.clone(),
                    also: also.clone(),
                },
            }),
            tail: launch.tail.clone(),
        }
    }

    fn at_work(&self) -> Option<&str> {
        match &self.agent.idle {
            manifest::agent::Idle::Signature(signature) => Some(signature),
            manifest::agent::Idle::Silence => None,
        }
    }

    fn exhausted(&self) -> Option<&str> {
        self.agent.exhausted.as_deref()
    }

    fn signed_out(&self) -> Option<SignedOut<'_>> {
        self.agent.signed_out.as_ref().map(|signed_out| SignedOut {
            ends: &signed_out.ends,
            then: signed_out.then.as_deref(),
            // Read in every record a session leaves, which is what a
            // harness that does not draw its conversation again says.
            read_in: ReadIn::Every,
        })
    }

    fn channel(&self) -> store::Channel {
        self.agent.ask.channel
    }

    fn tools_named(&self) -> &str {
        &self.agent.tools
    }

    /// The manifest's, which was read for this platform already.
    fn environment(&self, _platform: Platform) -> Vec<(&str, &str)> {
        self.agent
            .environment
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect()
    }

    fn written_in_worktree(&self) -> Vec<&str> {
        self.agent.excluded.iter().map(String::as_str).collect()
    }

    fn kept_in(&self, home: &Path) -> store::Account {
        store::Account::new(
            self.id,
            self.agent
                .account
                .iter()
                .map(|path| (path.key.clone(), home.join(&path.mounted))),
        )
    }

    fn directory<'a>(&self, account: &'a store::Account) -> &'a Path {
        account.path(&self.first().key).unwrap_or(Path::new(""))
    }

    fn account_inside(&self, account: &store::Account, home: &Path) -> Vec<(PathBuf, PathBuf)> {
        self.agent
            .account
            .iter()
            .filter_map(|path| {
                let held = account.path(&path.key)?;
                Some((held.to_owned(), home.join(&path.mounted)))
            })
            .collect()
    }

    fn judged(&self, account: &store::Account) -> Vec<(PathBuf, Broken)> {
        self.agent
            .account
            .iter()
            .map(|path| {
                let held = account.path(&path.key).unwrap_or(Path::new(""));
                let missing = match path.file {
                    true => Broken::ConfigMissing,
                    false => Broken::HomeMissing,
                };
                (held.to_owned(), missing)
            })
            .collect()
    }

    fn root(&self) -> &str {
        self.first()
            .mounted
            .to_str()
            .expect("a manifest is read as UTF-8")
    }

    /// The first credentials file of the first path, or the directory itself
    /// for an account with none — which is no file, so nothing is joined.
    fn login(&self) -> &str {
        self.first()
            .credentials
            .first()
            .and_then(|path| path.to_str())
            .unwrap_or_default()
    }

    fn configuration(&self) -> Vec<PathBuf> {
        self.configured_files()
    }

    /// What `write-configuration` answers, given each configuration file with
    /// the account's own servers taken out — see [`serverless`] — and the
    /// servers the Conversation attached; and the session-only settings, the
    /// export's where it gives any and the manifest's otherwise, as
    /// [`SETTINGS_FILE`].
    ///
    /// **An export that fails is a root with none of the configuration in it**,
    /// said in the log: the session starts on the account's login, its memory
    /// and the manifest's settings, rather than not at all.
    fn configured(
        &self,
        found: Vec<Option<Vec<u8>>>,
        servers: &[AttachedServer],
    ) -> Vec<(PathBuf, Vec<u8>)> {
        let found = self
            .configured_files()
            .into_iter()
            .zip(found)
            .map(|(path, contents)| world::Found {
                path: path.to_string_lossy().into_owned(),
                contents: contents.map(|contents| serverless(&contents)),
            })
            .collect();
        let servers = servers
            .iter()
            .map(|server| world::Server {
                name: server.name().to_owned(),
                url: server.url().to_owned(),
                headers: server.headers().to_vec(),
            })
            .collect();

        let (mut files, settings) = match self.component.configure(found, servers) {
            Ok(configuration) => (
                configuration
                    .files
                    .into_iter()
                    .filter_map(|file| {
                        let path = manifest::relative_path(&file.path)?;
                        Some((path, file.contents))
                    })
                    .collect(),
                configuration.settings,
            ),
            Err(why) => {
                tracing::error!(
                    harness = self.agent.name,
                    why,
                    "a plugin harness's write-configuration failed, so its session's root holds \
                     none of the account's configuration"
                );
                (Vec::new(), None)
            }
        };

        // The settings the line names are written either way: the line names
        // the file whatever the export answered.
        let settings = settings.or_else(|| {
            self.agent
                .launch
                .settings
                .as_ref()
                .map(|settings| settings.to_string())
        });
        if let Some(settings) = settings {
            files.push((PathBuf::from(SETTINGS_FILE), settings.into_bytes()));
        }

        files
    }

    fn instructions(&self) -> PathBuf {
        self.agent.instructions.clone()
    }

    fn memory(&self, _place: Option<&Place>) -> Vec<PathBuf> {
        self.first().memory.clone()
    }

    fn unshared(&self) -> Vec<&str> {
        self.first()
            .memory
            .iter()
            .filter_map(|path| path.to_str())
            .collect()
    }

    /// Nothing yet: a plugin harness's memory does not cross a link.
    fn synced(
        &self,
        _place: Option<&Place>,
        _worktree: Option<&str>,
        _sessions: &[String],
    ) -> Vec<Part> {
        Vec::new()
    }

    /// Its configuration files, which a root is composed from.
    fn mirrored(&self) -> Vec<PathBuf> {
        self.configured_files()
            .into_iter()
            .map(|file| self.first().mounted.join(file))
            .collect()
    }

    fn mirroring(&self, account: &store::Account) -> Vec<Option<Vec<u8>>> {
        self.configured_files()
            .into_iter()
            .map(|file| std::fs::read(self.directory(account).join(file)).ok())
            .collect()
    }

    /// Nowhere yet: its record is read by its `parse-transcript` export.
    fn search(
        &self,
        _account: &Path,
        _memory: bool,
        _home: &Home,
        _session: &str,
        _worktree: Option<&Path>,
        _launched: SystemTime,
    ) -> Search {
        Search::Nowhere
    }

    fn carried<'a>(
        &'a self,
        _account: &'a Path,
        _memory: bool,
        _session: &'a str,
        _worktree: &'a Path,
        _home: &'a Home,
    ) -> Carrying<'a> {
        Box::pin(async { None })
    }

    /// The model the manifest names for `role`, or its default.
    fn model(&self, role: store::Role) -> &str {
        self.agent
            .models
            .roles
            .iter()
            .find(|(declared, _)| *declared == role)
            .map_or(&self.agent.models.default, |(_, model)| model)
    }
}

/// `contents`, one of an account's configuration files, with the account's
/// own MCP servers taken out of it — ADR-0011's rule, and the host's rather
/// than the export's.
///
/// **By the keys the harnesses met so far keep them under**: a JSON object's
/// `mcpServers`, `mcp_servers` and `mcp`, and a TOML table's `mcp_servers`, at
/// the top. A file that is neither is handed over as it is.
pub(crate) fn serverless(contents: &[u8]) -> Vec<u8> {
    if let Ok(serde_json::Value::Object(mut object)) = serde_json::from_slice(contents) {
        for key in ["mcpServers", "mcp_servers", "mcp"] {
            object.remove(key);
        }
        return serde_json::to_vec_pretty(&object).unwrap_or_default();
    }

    if let Some(mut table) = std::str::from_utf8(contents)
        .ok()
        .and_then(|text| text.parse::<toml::Table>().ok())
    {
        table.remove("mcp_servers");
        return toml::to_string(&table).unwrap_or_default().into_bytes();
    }

    contents.to_vec()
}

/// A plugin's component, standing on a thread of its own with a runtime of its
/// own: the trait's answers are asked from blocking code and async code alike,
/// and the component's exports are async, so the calls are carried across to
/// the one place they can be awaited and the answer carried back.
struct Component {
    calls: mpsc::Sender<Call>,
}

/// One call into the component, and where its answer goes.
enum Call {
    Configure {
        found: Vec<world::Found>,
        servers: Vec<world::Server>,
        answered: mpsc::Sender<Result<world::Configuration, String>>,
    },
}

impl Component {
    fn standing(mut loaded: Loaded) -> Self {
        let (calls, taken) = mpsc::channel::<Call>();

        let started = std::thread::Builder::new()
            .name("plugin-harness".to_owned())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };

                // Each call ends here when every handle on it has gone, which
                // is the harness being dropped.
                for call in taken {
                    match call {
                        Call::Configure {
                            found,
                            servers,
                            answered,
                        } => {
                            let configuration = runtime.block_on(async {
                                let exports =
                                    world::Agent::new(&mut loaded.store, &loaded.instance)
                                        .map_err(|error| error.to_string())?;
                                loaded.store.set_epoch_deadline(
                                    CALLING
                                        .div_duration_f64(crate::plugins::component::TICK)
                                        .ceil() as u64,
                                );
                                let configuration = exports
                                    .call_write_configuration(&mut loaded.store, &found, &servers)
                                    .await
                                    .map_err(|error| crate::plugins::component::worded(&error));
                                loaded.store.set_epoch_deadline(u64::MAX);
                                configuration
                            });
                            let _ = answered.send(configuration);
                        }
                    }
                }
            });

        if let Err(error) = started {
            tracing::error!(error = ?error, "a plugin harness's component couldn't be started");
        }

        Component { calls }
    }

    /// `write-configuration`'s answer, or why there is none.
    fn configure(
        &self,
        found: Vec<world::Found>,
        servers: Vec<world::Server>,
    ) -> Result<world::Configuration, String> {
        let (answered, answer) = mpsc::channel();

        self.calls
            .send(Call::Configure {
                found,
                servers,
                answered,
            })
            .map_err(|_| "its component isn't running".to_owned())?;

        answer
            .recv()
            .map_err(|_| "its component stopped".to_owned())?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_account_s_own_servers_are_taken_out_of_json() {
        let found = br#"{"model":"large","mcpServers":{"own":{"url":"http://own"}}}"#;

        let left: serde_json::Value = serde_json::from_slice(&serverless(found)).unwrap();

        assert_eq!(left, serde_json::json!({ "model": "large" }));
    }

    #[test]
    fn the_account_s_own_servers_are_taken_out_of_toml() {
        let found = b"model = \"large\"\n\n[mcp_servers.own]\nurl = \"http://own\"\n";

        let left = String::from_utf8(serverless(found)).unwrap();

        assert_eq!(left, "model = \"large\"\n");
    }

    #[test]
    fn a_file_that_is_neither_is_handed_over_as_it_is() {
        assert_eq!(serverless(b"plain = = text"), b"plain = = text");
    }
}
