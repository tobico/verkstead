//! A harness a plugin declares: the sixth implementation of [`Harness`], and
//! the only one a third-party harness is ever (ADR-0030, *Agent*).
//!
//! **Built from what the plugin's manifest says and what its component
//! exports.** Everything about the harness that is data is read off its
//! `agent` provision — see [`crate::plugins::manifest::agent`] — and answered
//! here as the built-ins answer it out of their own modules. The three things
//! that are code are the component's exports, and this calls them with what the
//! host holds: `write-configuration`, which is what a root's configuration
//! is, and `parse-transcript`, which is what its sessions' Transcripts are
//! read out of — see [`Reader`].
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
//! paths after it are given to a session beside it — see [`Beside`].

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

use verkstead_render::{Broken, ProfileSaved};

use super::{
    Beside, Carrying, Declared, Harness, Install, Line, ReadIn, Resume, SETTINGS_FILE, SignedOut,
};
use crate::logins::Speaking;
use crate::platform::Platform;
use crate::plugins::component::worlds::category::agent as world;
use crate::plugins::component::{Again, Loaded};
use crate::plugins::manifest::agent::Synced;
use crate::plugins::manifest::{self, Agent};
use crate::sandbox::Home;
use crate::sandbox::root::{Part, Place, Whose};
use crate::settings::AttachedServer;
use crate::store;
use crate::transcript::{Carried, Search};

/// What the path labelled `label` is refused as on a save, a `file` or a
/// directory — see [`Declared::refused`]: naming the field the manifest
/// labelled, so that with two paths of one kind the refusal says which.
///
/// **But a directory kept under [`store::HOME`] is a home**, refused in the
/// words a home always has been, which a built-in's Profiles kept when it
/// became a plugin.
fn refused(key: &str, label: &str, file: bool) -> [ProfileSaved; 3] {
    if key == store::HOME && !file {
        return super::HOME_REFUSED;
    }

    [
        ProfileSaved::PathNotAbsolute(label.to_owned()),
        ProfileSaved::PathMissing(label.to_owned()),
        match file {
            true => ProfileSaved::PathNotAFile(label.to_owned()),
            false => ProfileSaved::PathNotADirectory(label.to_owned()),
        },
    ]
}

/// How long one call into a plugin's component may take before it is stopped.
const CALLING: Duration = Duration::from_secs(10);

/// A harness declared by a plugin: its id, the `agent` its manifest provides,
/// its mark, and its component standing.
pub(crate) struct Plugged {
    id: store::AgentType,
    agent: Agent,
    mark: String,
    component: Component,

    /// How its login goes, where the manifest declares one.
    speaking: Option<Speaking>,

    /// What starts the component again to read a session's record — see
    /// [`Reader`].
    again: Again,

    /// Where each of its paths is kept in a home, said from `~`.
    found_at: Vec<String>,
}

impl Plugged {
    /// The harness `agent` declares, under the plugin `id`, drawn with `mark`
    /// and run by the component `loaded`.
    pub(crate) fn new(id: store::AgentType, agent: Agent, mark: String, loaded: Loaded) -> Self {
        Plugged {
            id,
            speaking: agent
                .login
                .as_ref()
                .map(|login| Speaking::exported(&agent.name, login, loaded.again.clone())),
            found_at: agent
                .account
                .iter()
                .map(|path| format!("~/{}", path.mounted.to_string_lossy()))
                .collect(),
            agent,
            mark,
            again: loaded.again.clone(),
            component: Component::standing(loaded),
        }
    }

    /// The `agent` it was declared as, which is what says whether a plugin
    /// read again is the same harness.
    pub(crate) fn declared(&self) -> &Agent {
        &self.agent
    }

    /// Its id, which is its plugin's uuid.
    pub(crate) fn id(&self) -> store::AgentType {
        self.id
    }

    /// What starts its component, which says whether a plugin read again is
    /// running the same code — see [`Again::same_as`].
    pub(crate) fn component(&self) -> &Again {
        &self.again
    }

    /// The first of the account's paths, which the account is said from.
    fn first(&self) -> &manifest::agent::AccountPath {
        &self.agent.account[0]
    }

    /// What reads the record of `session`, launched in `worktree` at
    /// `launched`, under the account at `account` — which is the directory the
    /// account is where the Profile shares its memory and the session's own
    /// root on the host where it does not, the built-ins' rule said from the
    /// first path.
    ///
    /// **`session` is looked for by only where the line named it**: a harness
    /// whose line names no `{session}` was never told it, and is found by its
    /// Worktree and its launch until its record says what it chose.
    fn reader(
        &self,
        account: &Path,
        memory: bool,
        home: &Home,
        session: &str,
        worktree: Option<&Path>,
        launched: SystemTime,
    ) -> Reader {
        Reader {
            again: self.again.clone(),
            account: match memory {
                true => account.to_owned(),
                false => home.built().join(&self.first().mounted),
            },
            session: self
                .agent
                .launch
                .session
                .as_ref()
                .map(|_| session.to_owned()),
            worktree: worktree.map(Path::to_owned),
            launched,
        }
    }

    /// Of the first path's allowlist, what is neither its login nor its
    /// memory: the account's configuration, handed to `write-configuration`.
    fn configured_files(&self) -> Vec<PathBuf> {
        configured(self.first())
    }
}

/// Of `path`'s allowlist, what is neither its login nor its memory: the
/// account's configuration in it.
fn configured(path: &manifest::agent::AccountPath) -> Vec<PathBuf> {
    path.allowlist
        .iter()
        .filter(|file| {
            !path.credentials.contains(file)
                && !path.memory.iter().any(|memory| memory.path == **file)
        })
        .cloned()
        .collect()
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
                refused: refused(&path.key, &path.label, path.file),
                within: &[],
            })
            .collect()
    }

    /// Each path where it is mounted, said from `~` as the trait says it.
    fn found_at(&self) -> Vec<&str> {
        self.found_at.iter().map(String::as_str).collect()
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
            worktree: launch.worktree.clone(),
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

    fn beside(&self, account: &store::Account) -> Vec<Beside> {
        self.agent.account[1..]
            .iter()
            .filter_map(|path| {
                Some(Beside {
                    host: account.path(&path.key)?.to_owned(),
                    mounted: path.mounted.clone(),
                    file: path.file,
                    credentials: path.credentials.clone(),
                    memory: path
                        .memory
                        .iter()
                        .map(|memory| memory.path.clone())
                        .collect(),
                    configuration: configured(path),
                })
            })
            .collect()
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

    /// The first path's directory, and each directory after it, which a root
    /// is built in place of too — see [`Beside`].
    fn built(&self, _memory: bool) -> Vec<&str> {
        self.agent
            .account
            .iter()
            .enumerate()
            .filter(|(at, path)| *at == 0 || !path.file)
            .filter_map(|(_, path)| path.mounted.to_str())
            .collect()
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
        self.first()
            .memory
            .iter()
            .map(|memory| memory.path.clone())
            .collect()
    }

    fn unshared(&self) -> Vec<&str> {
        self.first()
            .memory
            .iter()
            .filter_map(|memory| memory.path.to_str())
            .collect()
    }

    /// Each of the first path's memory directories the manifest says is
    /// synced, by the host's rule it names, under the part it names or its own
    /// path as its label.
    fn synced(
        &self,
        _place: Option<&Place>,
        worktree: Option<&str>,
        sessions: &[String],
    ) -> Vec<Part> {
        self.first()
            .memory
            .iter()
            .filter_map(|memory| {
                let whose = match memory.synced.as_ref()? {
                    Synced::Everything => Whose::Everything,
                    Synced::Worktree => Whose::Naming(worktree.unwrap_or_default().to_owned()),
                    Synced::Sessions => Whose::Called(sessions.to_vec()),
                    Synced::Database(name) => Whose::Database(name.clone()),
                };
                Some(Part {
                    label: memory
                        .part
                        .clone()
                        .unwrap_or_else(|| memory.path.to_string_lossy().into_owned())
                        .into(),
                    inside: memory.path.clone(),
                    whose,
                })
            })
            .collect()
    }

    /// Its configuration files, which a root is composed from.
    fn mirrored(&self) -> Vec<PathBuf> {
        self.configured_files()
            .into_iter()
            .map(|file| self.first().mounted.join(file))
            .collect()
    }

    /// What `write-configuration` answers for each of them, handed the
    /// account's configuration and no servers: what a root built from the
    /// mirror would hold, and nothing of how the human works beyond it. A file
    /// the export answers nothing for is none in the mirror.
    fn mirroring(&self, account: &store::Account) -> Vec<Option<Vec<u8>>> {
        let files = self.configured_files();
        let found = files
            .iter()
            .map(|file| std::fs::read(self.directory(account).join(file)).ok())
            .collect();
        let mut answered = self.configured(found, &[]);

        files
            .iter()
            .map(|file| {
                let at = answered.iter().position(|(path, _)| path == file)?;
                Some(answered.swap_remove(at).1)
            })
            .collect()
    }

    /// What its `parse-transcript` export answers for the session — where the
    /// record is kept being the export's to find, by the id the session was
    /// named or, for a harness that names itself, by its Worktree and its
    /// launch. A session of the latter with no Worktree has nothing to be
    /// found by.
    fn search(
        &self,
        account: &Path,
        memory: bool,
        home: &Home,
        session: &str,
        worktree: Option<&Path>,
        launched: SystemTime,
    ) -> Search {
        let reader = self.reader(account, memory, home, session, worktree, launched);
        if reader.session.is_none() && reader.worktree.is_none() {
            return Search::Nowhere;
        }
        Search::Parsed(reader)
    }

    /// As far as `parse-transcript` reads the record of `session` now, before
    /// the resumed session adds to it; none where it finds none, or can't read
    /// it. Looked for by `session` whatever the line names, since that is the
    /// id the record gave or the session was named.
    fn carried<'a>(
        &'a self,
        account: &'a Path,
        memory: bool,
        session: &'a str,
        worktree: &'a Path,
        home: &'a Home,
    ) -> Carrying<'a> {
        let reader = self.reader(
            account,
            memory,
            home,
            session,
            Some(worktree),
            SystemTime::UNIX_EPOCH,
        );

        Box::pin(async move {
            let mut entries = Entries::resuming(reader, session.to_owned(), None);
            let Ok(Some(answered)) = entries.answered().await else {
                return None;
            };
            Some(Carried::Entries {
                session: answered.session.unwrap_or_else(|| session.to_owned()),
                cursor: answered.cursor,
            })
        })
    }

    /// Where the manifest declares a login: its line run by the host, and
    /// what the harness writes read by the `login` export — see
    /// [`crate::logins::Dialogue::Exported`].
    fn speaking(&self) -> Option<&Speaking> {
        self.speaking.as_ref()
    }

    /// The model the manifest names for `role`, or its default.
    fn model(&self, role: &store::Role) -> &str {
        self.agent
            .models
            .roles
            .iter()
            .find(|(declared, _)| declared == role)
            .map_or(&self.agent.models.default, |(_, model)| model)
    }

    /// The manifest's list, in the order it declares them.
    fn models(&self) -> Vec<(&str, &str)> {
        self.agent
            .models
            .list
            .iter()
            .map(|model| (model.id.as_str(), model.name.as_str()))
            .collect()
    }

    /// The section the manifest wrote under `ask.guide`.
    fn running(&self) -> &str {
        &self.agent.ask.guide
    }

    /// None where the manifest declares neither commands nor instructions.
    fn install(&self) -> Option<Install<'_>> {
        installing(&self.agent.install)
    }
}

/// A manifest's `install` as the wizard reads it — None where it declares
/// neither commands nor instructions.
pub(crate) fn installing(
    install: &crate::plugins::manifest::agent::Install,
) -> Option<Install<'_>> {
    if *install == Default::default() {
        return None;
    }

    Some(Install {
        instructions: install.instructions.as_deref(),
        commands: &install.commands,
        npm: install.npm.as_deref(),
        brew: install.brew.as_deref(),
        cask: install.cask.as_deref(),
        nix: install.nix.as_deref(),
    })
}

/// Where a session of a plugin harness keeps its record, as its
/// `parse-transcript` is handed it: the account's root on the host, which the
/// export is handed as [`crate::plugins::component::ACCOUNT`], the session's id
/// where its line named one, and the Worktree and the moment it was launched
/// in, which a harness that names itself is found by.
#[derive(Clone)]
pub(crate) struct Reader {
    again: Again,
    account: PathBuf,
    session: Option<String>,
    worktree: Option<PathBuf>,
    launched: SystemTime,
}

#[cfg(test)]
impl Reader {
    /// A reader of the record a built-in plugin's harness `agent_type` keeps
    /// under `account`, looked for by `session` or by `worktree` and
    /// `launched`: what a session sharing its Profile's memory is read by.
    pub(crate) fn of_built_in(
        agent_type: store::AgentType,
        account: &Path,
        session: Option<&str>,
        worktree: Option<&Path>,
        launched: SystemTime,
    ) -> Reader {
        Reader {
            again: crate::harness::embedded(agent_type)
                .expect("a built-in plugin's harness")
                .again
                .clone(),
            account: account.to_owned(),
            session: session.map(str::to_owned),
            worktree: worktree.map(Path::to_owned),
            launched,
        }
    }
}

/// A session's record as its plugin harness's `parse-transcript` reads it,
/// followed as far as it has been taken.
///
/// **The export answers only what is past the cursor it is handed**, and hands
/// back the next one, so following it is keeping the last cursor and writing
/// what each answer holds — each entry as the line
/// [`verkstead_render::Entry::line`] makes, which the render crate draws. A
/// record that runs to megabytes is read once rather than every poll.
///
/// **And the id an answer gives is the session's from then on.** A harness
/// that names itself is looked for by its Worktree until it says what it
/// chose; it is then asked for by that id, and the id is written down beside
/// the session once — see [`Entries::found_itself`].
///
/// **The component reads in a store of its own**, started with the account's
/// root granted read-only beside what its manifest was granted — see
/// [`Again`] — and kept for the life of the following.
///
/// **An export that fails is the end of the following**, said once on the
/// Transcript as [`verkstead_render::Entry::Unreadable`]: a trap leaves a
/// component instance that can't be entered again, and one that answered
/// something that isn't an answer would answer it again. The session is
/// nothing to it.
pub(crate) struct Entries {
    reader: Reader,
    loaded: Option<Loaded>,
    cursor: Option<String>,

    /// An id an answer gave that is not written down yet.
    named: Option<String>,

    over: bool,
}

impl Entries {
    /// The following of `reader`'s record from the top.
    pub(crate) fn of(reader: Reader) -> Entries {
        Entries {
            reader,
            loaded: None,
            cursor: None,
            named: None,
            over: false,
        }
    }

    /// The following of the record of `session`, carried on from `cursor` —
    /// a resumed session's, whose record is known by the id it already has.
    pub(crate) fn resuming(mut reader: Reader, session: String, cursor: Option<String>) -> Entries {
        reader.session = Some(session);
        Entries {
            cursor,
            ..Entries::of(reader)
        }
    }

    /// The lines of what the record holds past what was taken last time.
    pub(crate) async fn take(&mut self) -> Vec<String> {
        if self.over {
            return Vec::new();
        }

        match self.answered().await {
            // No record yet, or no root yet to find one in.
            Ok(None) => Vec::new(),
            Ok(Some(answered)) => {
                if let Some(session) = answered.session
                    && self.reader.session.as_ref() != Some(&session)
                {
                    self.reader.session = Some(session.clone());
                    self.named = Some(session);
                }
                self.cursor = Some(answered.cursor);

                answered
                    .entries
                    .into_iter()
                    .map(|entry| entered(entry).line())
                    .collect()
            }
            Err(why) => {
                tracing::warn!(
                    account = %self.reader.account.display(),
                    session = self.reader.session,
                    why,
                    "a plugin harness's parse-transcript failed, so its session's Transcript \
                     stops here"
                );
                self.over = true;
                self.loaded = None;
                vec![
                    verkstead_render::Entry::Unreadable {
                        why: format!("Its harness couldn't read it: {why}."),
                    }
                    .line(),
                ]
            }
        }
    }

    /// The id the record gave the session, the once: on the take that first
    /// found it, and never after. What the session is written down as — see
    /// [`crate::transcript`].
    pub(crate) fn found_itself(&mut self) -> Option<String> {
        self.named.take()
    }

    /// What `parse-transcript` answers for the session now, past the cursor:
    /// none where there is no record of it, or no account root yet to look in.
    async fn answered(&mut self) -> Result<Option<world::Transcript>, String> {
        let account = &self.reader.account;

        let loaded = match &mut self.loaded {
            Some(loaded) => loaded,
            None => {
                // A root the session hasn't been built into yet: nothing to
                // look in, and looked in again on the next poll.
                if !tokio::fs::metadata(account)
                    .await
                    .is_ok_and(|found| found.is_dir())
                {
                    return Ok(None);
                }
                let loaded = self
                    .reader
                    .again
                    .reading(account)
                    .await
                    .map_err(|trouble| trouble.why.trim_end_matches('.').to_owned())?;
                self.loaded.insert(loaded)
            }
        };

        let looking = world::Looking {
            session: self.reader.session.clone(),
            worktree: self
                .reader
                .worktree
                .as_ref()
                .map(|worktree| worktree.to_string_lossy().into_owned()),
            launched: self
                .reader
                .launched
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |since| since.as_millis() as u64),
            cursor: self.cursor.clone(),
        };

        let exports = world::Agent::new(&mut loaded.store, &loaded.instance)
            .map_err(|error| crate::plugins::component::worded(&error))?;
        loaded.store.set_epoch_deadline(
            CALLING
                .div_duration_f64(crate::plugins::component::TICK)
                .ceil() as u64,
        );
        let answered = within_time(
            exports.call_parse_transcript(
                &mut loaded.store,
                crate::plugins::component::ACCOUNT,
                &looking,
            ),
            failed,
        )
        .await;
        loaded.store.set_epoch_deadline(u64::MAX);
        answered
    }
}

/// What `call` answers, or why it failed, worded by `worded` — and its
/// running out of time where it has not answered within [`CALLING`].
///
/// **The epoch deadline alone is not a deadline.** It stops a component only
/// while the component's own code runs, and one waiting in a call to the host
/// — a sleep, or a request it was granted the network for — runs none: it
/// would wait there for ever, and a `write-configuration` doing so would hold
/// every later root of its harness behind it. So the call is also held to the
/// clock on the wall, and dropped where it runs over; the instance it was in is
/// never entered again, as one that trapped is not.
async fn within_time<T>(
    call: impl Future<Output = wasmtime::Result<T>>,
    worded: fn(&wasmtime::Error) -> String,
) -> Result<T, String> {
    match tokio::time::timeout(CALLING, call).await {
        Ok(answered) => answered.map_err(|error| worded(&error)),
        Err(_) => Err(format!("it took longer than {} seconds", CALLING.as_secs())),
    }
}

/// Why a call into a component failed, as the end of a sentence: a trap as the
/// trap alone, for the reason [`crate::plugins::component`] words one that way.
fn failed(error: &wasmtime::Error) -> String {
    match error.downcast_ref::<wasmtime::Trap>() {
        Some(wasmtime::Trap::Interrupt) => {
            format!("it took longer than {} seconds", CALLING.as_secs())
        }
        Some(trap) => format!("it stopped: {trap}"),
        None => crate::plugins::component::worded(error),
    }
}

/// A plugin harness's `login` export, standing for one harness of a login: an
/// instance of the component of its own, started afresh, since what the export
/// answers builds on what it was handed before — see
/// [`crate::logins::Dialogue::Exported`].
pub(crate) struct Exchange {
    loaded: Loaded,
}

impl Exchange {
    /// `again`'s component, started afresh for a login.
    pub(crate) async fn started(again: &Again) -> Result<Exchange, String> {
        let loaded = again
            .afresh()
            .await
            .map_err(|trouble| trouble.why.trim_end_matches('.').to_owned())?;
        Ok(Exchange { loaded })
    }

    /// What `login` answers, handed what the harness has written since it was
    /// last asked.
    pub(crate) async fn step(&mut self, written: &str) -> Result<world::LoginStep, String> {
        let loaded = &mut self.loaded;
        let exports = world::Agent::new(&mut loaded.store, &loaded.instance)
            .map_err(|error| crate::plugins::component::worded(&error))?;
        loaded.store.set_epoch_deadline(
            CALLING
                .div_duration_f64(crate::plugins::component::TICK)
                .ceil() as u64,
        );
        let step = within_time(exports.call_login(&mut loaded.store, written), failed).await;
        loaded.store.set_epoch_deadline(u64::MAX);
        step
    }
}

/// One entry the export answered, as the render crate draws it.
fn entered(entry: world::Entry) -> verkstead_render::Entry {
    use verkstead_render::{Entry, Voice};

    match entry {
        world::Entry::Said(said) => Entry::Said {
            voice: match said.voice {
                world::Voice::Agent => Voice::Agent,
                world::Voice::Reasoning => Voice::Reasoning,
                world::Voice::Put => Voice::Put,
            },
            markdown: said.markdown,
        },
        world::Entry::ToolCall(call) => Entry::ToolCall {
            name: call.name,
            call: call.call,
            about: call.about,
            input: call.input,
        },
        world::Entry::ToolResult(result) => Entry::ToolResult {
            call: result.call,
            failed: result.failed,
            text: result.text,
        },
        world::Entry::Notice(notice) => Entry::Notice {
            kind: notice.kind,
            line: notice.line,
        },
        world::Entry::Unknown(unknown) => Entry::Unknown {
            kind: unknown.kind,
            line: unknown.line,
        },
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
                                let configuration = within_time(
                                    exports.call_write_configuration(
                                        &mut loaded.store,
                                        &found,
                                        &servers,
                                    ),
                                    crate::plugins::component::worded,
                                )
                                .await;
                                loaded.store.set_epoch_deadline(u64::MAX);
                                configuration
                            });

                            // A component that trapped, or ran out of time,
                            // is never entered again, so the next call is
                            // answered by one started afresh — as a Tail's
                            // reader is — rather than by the one that failed.
                            if configuration.is_err() {
                                match runtime.block_on(loaded.again.afresh()) {
                                    Ok(fresh) => loaded = fresh,
                                    Err(trouble) => tracing::error!(
                                        trouble = %trouble,
                                        "a plugin harness's component couldn't be started again",
                                    ),
                                }
                            }

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

        // Twice the call's own time: enough for one that ran out of it and the
        // component started afresh after it, and never a wait without end.
        answer
            .recv_timeout(CALLING * 2)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => {
                    format!("it took longer than {} seconds", (CALLING * 2).as_secs())
                }
                mpsc::RecvTimeoutError::Disconnected => "its component stopped".to_owned(),
            })?
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

    /// The stub harness's component finds a session's record under the name
    /// it is handed for the account, wherever the account is on the host — a
    /// `C:\…` directory on Windows among them.
    #[tokio::test]
    async fn parse_transcript_reads_the_record_under_the_account_it_is_handed() {
        let account = tempfile::tempdir().unwrap();
        std::fs::create_dir(account.path().join("memory")).unwrap();
        std::fs::write(
            account.path().join("memory").join("s1.jsonl"),
            "{\"said\":\"agent\",\"markdown\":\"Reading the brief.\"}\n",
        )
        .unwrap();

        let reader = named("s1", stub().await.again, account.path());

        let lines = Entries::of(reader).take().await;

        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("Reading the brief."), "{lines:?}");
    }

    /// A reader of the session named `session`, under `account`.
    fn named(session: &str, again: Again, account: &Path) -> Reader {
        Reader {
            again,
            account: account.to_owned(),
            session: Some(session.to_owned()),
            worktree: None,
            launched: SystemTime::UNIX_EPOCH,
        }
    }

    /// The stub harness's component, loaded.
    async fn stub() -> Loaded {
        let bytes = std::fs::read(env!("VERKSTEAD_STUB_HARNESS_WASM")).unwrap();
        crate::plugins::component::instantiate(&bytes, &[], None)
            .await
            .expect("the stub harness's component should load")
    }

    /// A poll after the first is handed the cursor the first answered, and
    /// what the component answers past it is all that is written: a record
    /// changed under what was read already would be read again from the top.
    #[tokio::test]
    async fn a_poll_after_the_first_reads_only_past_the_cursor() {
        let account = tempfile::tempdir().unwrap();
        let memory = account.path().join("memory");
        std::fs::create_dir(&memory).unwrap();
        let first = "{\"said\":\"agent\",\"markdown\":\"First.\"}\n";
        std::fs::write(memory.join("s1.jsonl"), first).unwrap();

        let mut entries = Entries::of(named("s1", stub().await.again, account.path()));
        let lines = entries.take().await;
        assert_eq!(lines.len(), 1, "{lines:?}");

        // The line already read, written over with one of the same length: a
        // reading from the top would answer it again, as something else.
        let changed = first.replace("First.", "Again.");
        std::fs::write(
            memory.join("s1.jsonl"),
            format!("{changed}{{\"said\":\"agent\",\"markdown\":\"Second.\"}}\n"),
        )
        .unwrap();

        let lines = entries.take().await;
        assert_eq!(lines.len(), 1, "only the line past the cursor: {lines:?}");
        assert!(lines[0].contains("Second."), "{lines:?}");
    }

    /// A harness that names itself is found by its Worktree, among a record
    /// of another Worktree's written at the same moment; and the id its
    /// record gave is the session's, found the once.
    #[tokio::test]
    async fn a_record_that_names_itself_is_found_by_its_worktree() {
        let account = tempfile::tempdir().unwrap();
        let memory = account.path().join("memory");
        std::fs::create_dir(&memory).unwrap();
        std::fs::write(
            memory.join("theirs.jsonl"),
            "{\"session\":\"theirs\",\"cwd\":\"/srv/tables\"}\n\
             {\"said\":\"agent\",\"markdown\":\"Not this one.\"}\n",
        )
        .unwrap();
        std::fs::write(
            memory.join("mine.jsonl"),
            "{\"session\":\"mine\",\"cwd\":\"/srv/counter\"}\n\
             {\"said\":\"agent\",\"markdown\":\"This one.\"}\n",
        )
        .unwrap();

        let mut entries = Entries::of(Reader {
            again: stub().await.again,
            account: account.path().to_owned(),
            session: None,
            worktree: Some(PathBuf::from("/srv/counter")),
            launched: SystemTime::now() - Duration::from_secs(5),
        });

        let lines = entries.take().await;
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("This one."), "{lines:?}");
        assert_eq!(entries.found_itself().as_deref(), Some("mine"));

        entries.take().await;
        assert_eq!(entries.found_itself(), None, "the once");
    }

    /// The render suite's Codex rollout fixtures, as one rollout — and after
    /// them a line apiece of what it has no fixture of: the other three tool
    /// items, thinking with no summary, a message with a picture in it and one
    /// with nothing, an event naming no item or nothing at all, a line with no
    /// kind, and one that is not JSON.
    const ROLLOUT: &[&str] = &[
        r#"{"timestamp":"2026-08-30T08:11:27.307Z","ordinal":0,"type":"session_meta","payload":{"session_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","cwd":"/srv/worktrees/tables","originator":"codex_exec"}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.307Z","ordinal":1,"type":"event_msg","payload":{"type":"task_started","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","started_at":1788077487,"model_context_window":258400,"collaboration_mode_kind":"default"}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.311Z","ordinal":2,"type":"response_item","payload":{"type":"message","id":"msg_01a051b9-34cf-75b2-93a0-acaca50f73a3","role":"developer","content":[{"type":"input_text","text":"<skills_instructions>\n## Skills\nA skill is a set of instructions provided through a `SKILL.md` source."}]}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.311Z","ordinal":3,"type":"response_item","payload":{"type":"message","id":"msg_01a051b9-34cf-75b2-93a0-acb06c799843","role":"user","content":[{"type":"input_text","text":"<environment_context>\n  <cwd>/srv/worktrees/tables</cwd>\n  <shell>bash</shell>\n</environment_context>"}]}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.311Z","ordinal":4,"type":"world_state","payload":{"full":true,"state":{"agents_md":{},"apps_instructions":false}}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.311Z","ordinal":5,"type":"turn_context","payload":{"turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","cwd":"/srv/worktrees/tables"}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.316Z","ordinal":6,"type":"response_item","payload":{"type":"message","id":"msg_01a051b9-34d4-7612-95dc-1001c4fb2884","role":"user","content":[{"type":"input_text","text":"What is in the task list?"}],"internal_chat_message_metadata_passthrough":{"turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","create_time":1788077487.3162456}}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.316Z","ordinal":7,"type":"event_msg","payload":{"type":"item_completed","thread_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","item":{"type":"UserMessage","id":"01a051b9-34d4-7612-95dc-10185af451bf","content":[{"type":"text","text":"What is in the task list?","text_elements":[]}]},"started_at_ms":1788077487316,"completed_at_ms":1788077487316}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.335Z","ordinal":8,"type":"event_msg","payload":{"type":"item_completed","thread_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","item":{"type":"Reasoning","id":"rs-1","summary_text":["**Reading the task list**\n\nThe backlog is where to start."],"raw_content":[]},"started_at_ms":1788077487335,"completed_at_ms":1788077487335}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.335Z","ordinal":9,"type":"response_item","payload":{"type":"reasoning","id":"rs-1","summary":[{"type":"summary_text","text":"**Reading the task list**\n\nThe backlog is where to start."}],"content":null,"encrypted_content":null}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.336Z","ordinal":10,"type":"event_msg","payload":{"type":"item_completed","thread_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","item":{"type":"AgentMessage","id":"msg-1","content":[{"type":"Text","text":"I'll look at the **task list** first."}]},"started_at_ms":1788077487336,"completed_at_ms":1788077487336}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.336Z","ordinal":11,"type":"response_item","payload":{"type":"message","id":"msg-1","role":"assistant","content":[{"type":"output_text","text":"I'll look at the **task list** first."}]}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.337Z","ordinal":12,"type":"response_item","payload":{"type":"function_call","id":"fc_01a051b9-34e9-7e33-b5b0-7ec74bfd68c9","name":"exec_command","arguments":"{\"cmd\":\"ls .tasks\"}","call_id":"call-1"}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.369Z","ordinal":13,"type":"event_msg","payload":{"type":"item_completed","thread_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","item":{"type":"CommandExecution","id":"call-1","process_id":"45936","command":["/bin/bash","-lc","ls .tasks"],"cwd":"file:///srv/worktrees/tables","parsed_cmd":[{"type":"list_files","cmd":"ls .tasks","path":".tasks"}],"source":"unified_exec_startup","status":"completed","stdout":"04-render.md\n05-summaries.md\n","stderr":"","aggregated_output":"04-render.md\n05-summaries.md\n","exit_code":0,"duration":{"secs":0,"nanos":2425},"formatted_output":"04-render.md\n05-summaries.md\n"},"started_at_ms":1788077487369,"completed_at_ms":1788077487369}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.370Z","ordinal":14,"type":"response_item","payload":{"type":"function_call_output","id":"fco_01a051b9-350a-7a20-932c-94ffaad12aba","call_id":"call-1","output":"Chunk ID: 57be28\nProcess exited with code 0\nOutput:\n04-render.md\n05-summaries.md\n"}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.371Z","ordinal":15,"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":1200,"output_tokens":40,"total_tokens":1240},"model_context_window":258400}}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.388Z","ordinal":16,"type":"event_msg","payload":{"type":"item_completed","thread_id":"01a051b9-34c0-7e60-b078-f31f1f443ebe","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","item":{"type":"AgentMessage","id":"msg-2","content":[{"type":"Text","text":"Two files: `04-render.md` and `05-summaries.md`."}]},"started_at_ms":1788077487388,"completed_at_ms":1788077487388}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.388Z","ordinal":17,"type":"response_item","payload":{"type":"message","id":"msg-2","role":"assistant","content":[{"type":"output_text","text":"Two files: `04-render.md` and `05-summaries.md`."}]}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.390Z","ordinal":19,"type":"event_msg","payload":{"type":"task_complete","turn_id":"01a051b9-34c8-7bb2-ba8e-2969c9c4df12","last_agent_message":"Two files: `04-render.md` and `05-summaries.md`.","started_at":1788077487,"completed_at":1788077487,"duration_ms":85}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"CommandExecution","id":"call-9","command":["/bin/bash","-lc","cargo test"],"status":"failed","aggregated_output":"error: could not compile","exit_code":101}}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"McpToolCall","id":"mcp-1","server":"github","tool":"list_issues","arguments":{"query":"is:open label:bug"},"status":"failed","error":{"message":"the token is not authorised"}}}}"#,
        r#"{"timestamp":"2026-08-30T08:11:27.390Z","ordinal":20,"type":"seance","payload":{"heard":"a kind from a later version"}}"#,
        r#"{"type":"event_msg","payload":{"type":"stream_error","message":"retrying"}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"Telepathy","id":"tp-1","thought":"a kind from a later version"}}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","id":"msg-1","content":[{"type":"Text","text":"And the *rollout* was read too."}]}}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"FileChange","id":"fc-1","changes":{"src/a.rs":{"type":"update"},"src/b.rs":{"type":"add"}},"status":"completed","stdout":"Success."}}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"WebSearch","id":"ws-1","query":"toml inline tables","action":{"type":"search"},"results":null}}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"DynamicToolCall","id":"dt-1","namespace":"","tool":"lookup","arguments":{"description":"Look up every place the rate limiter is constructed, and every place its window is read, across the server and the CLI both"},"status":"completed","content_items":[{"type":"text","text":"found"}]}}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"CommandExecution","id":"call-10","command":["/bin/bash","-lc","cargo test --workspace"],"status":"completed","formatted_output":"ok","exit_code":0}}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"Reasoning","id":"rs-2","summary_text":[],"raw_content":["The window first.","Then the counter."]}}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"UserMessage","id":"um-2","content":[{"type":"image","url":"file:///tmp/a.png"},{"type":"text","text":" Like this."}]}}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","id":"msg-9","content":[]}}}"#,
        r#"{"type":"event_msg","payload":{"type":"item_completed"}}"#,
        r#"{"type":"event_msg","payload":{}}"#,
        r#"{"type":"compacted","payload":{"message":"The story so far."}}"#,
        r#"{"ordinal":30}"#,
        r#"not a line of JSON"#,
    ];

    /// The Worktree [`ROLLOUT`] names, and the id it gives itself.
    const ROLLOUT_WORKTREE: &str = "/srv/worktrees/tables";
    const ROLLOUT_SESSION: &str = "01a051b9-34c0-7e60-b078-f31f1f443ebe";

    /// `lines` as a rollout codex filed under `account`'s `sessions/` on `day`.
    fn filed(account: &Path, day: &str, name: &str, lines: &[&str]) -> PathBuf {
        let dir = account.join("sessions").join(day);
        std::fs::create_dir_all(&dir).unwrap();
        let rollout = dir.join(name);
        std::fs::write(
            &rollout,
            lines
                .iter()
                .map(|line| format!("{line}\n"))
                .collect::<String>(),
        )
        .unwrap();
        rollout
    }

    /// A reader of Codex's plugin, looking for `session` or by its Worktree.
    async fn codex(account: &Path, session: Option<&str>) -> Reader {
        Reader::of_built_in(
            store::AgentType::CODEX,
            account,
            session,
            Some(Path::new(ROLLOUT_WORKTREE)),
            SystemTime::now() - Duration::from_secs(60),
        )
    }

    /// Codex's rollout, read by its plugin, is drawn as the render crate draws
    /// the rollout itself: the same things said, the same tool calls and
    /// results, and the same lines folded away under the same names.
    #[tokio::test]
    async fn codexs_plugin_reads_a_rollout_into_what_the_render_crate_draws() {
        let account = tempfile::tempdir().unwrap();
        filed(
            account.path(),
            "2026/08/30",
            "rollout-2026-08-30T08-11-27-01a051b9-34c0-7e60-b078-f31f1f443ebe.jsonl",
            ROLLOUT,
        );

        let mut entries = Entries::of(codex(account.path(), None).await);
        let lines = entries.take().await;

        let read = verkstead_render::transcript_view(&lines);
        let drawn = verkstead_render::transcript_view(
            &ROLLOUT
                .iter()
                .map(|line| (*line).to_owned())
                .collect::<Vec<_>>(),
        );
        assert_eq!(read.turns, drawn.turns);
        assert_eq!(read.bookkeeping, drawn.bookkeeping);
        assert_eq!(entries.found_itself().as_deref(), Some(ROLLOUT_SESSION));
    }

    /// Codex names itself: its rollout is found by the Worktree its opening
    /// line names, among another Worktree's written the same day and what else
    /// codex keeps beside them — and once it has said which session it is, by
    /// that id, on whichever day it was filed.
    #[tokio::test]
    async fn codexs_plugin_finds_a_rollout_by_its_worktree_and_then_by_its_id() {
        let account = tempfile::tempdir().unwrap();
        let theirs = r#"{"type":"session_meta","payload":{"session_id":"theirs","cwd":"/srv/worktrees/counter"}}"#;
        let said = |text: &str| {
            format!(
                r#"{{"type":"event_msg","payload":{{"type":"item_completed","item":{{"type":"AgentMessage","id":"m","content":[{{"type":"Text","text":"{text}"}}]}}}}}}"#
            )
        };
        filed(
            account.path(),
            "2026/08/30",
            "rollout-theirs.jsonl",
            &[theirs, &said("Not this one.")],
        );
        filed(
            account.path(),
            "2026/08/30",
            "rollout-ours.jsonl.zst",
            &[ROLLOUT[0], &said("Nor this.")],
        );
        filed(
            account.path(),
            "2026/08/30",
            "rollout-ours.jsonl",
            &[ROLLOUT[0], &said("This one.")],
        );

        let spoken = |lines: Vec<String>| -> Vec<String> {
            lines
                .into_iter()
                .filter(|line| line.contains("\"said\""))
                .collect()
        };
        let mut entries = Entries::of(codex(account.path(), None).await);
        let lines = spoken(entries.take().await);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("This one."), "{lines:?}");
        assert_eq!(entries.found_itself().as_deref(), Some(ROLLOUT_SESSION));

        // Carried to another device, where the memory sync filed it another
        // day, and asked for by the id alone.
        let elsewhere = tempfile::tempdir().unwrap();
        let rollout = filed(
            elsewhere.path(),
            "2026/09/01",
            "rollout-carried.jsonl",
            &[ROLLOUT[0], &said("Carried.")],
        );
        filed(
            elsewhere.path(),
            "2026/09/02",
            "rollout-theirs.jsonl",
            &[theirs],
        );
        let mut reader = codex(elsewhere.path(), Some(ROLLOUT_SESSION)).await;
        reader.worktree = None;
        reader.launched = SystemTime::UNIX_EPOCH;
        let mut entries = Entries::of(reader);
        let lines = spoken(entries.take().await);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("Carried."), "{lines:?}");

        // And a poll after the first reads what was added since, and no more.
        let mut more = std::fs::OpenOptions::new()
            .append(true)
            .open(&rollout)
            .unwrap();
        std::io::Write::write_all(
            &mut more,
            format!("{}\n{{\"type\"", said("Then this.")).as_bytes(),
        )
        .unwrap();
        let lines = entries.take().await;
        assert_eq!(lines.len(), 1, "the line half-written waits: {lines:?}");
        assert!(lines[0].contains("Then this."), "{lines:?}");
    }

    /// A component waiting on the host runs none of its own code, which the
    /// epoch deadline is all about — so it is the clock on the wall that ends
    /// the call, and the Transcript says it ran out of time.
    #[tokio::test]
    async fn a_parse_transcript_waiting_on_the_host_runs_out_of_time() {
        let account = tempfile::tempdir().unwrap();
        std::fs::create_dir(account.path().join("memory")).unwrap();
        std::fs::write(
            account.path().join("memory").join("s1.jsonl"),
            "{\"sleep\":true}\n",
        )
        .unwrap();

        let reader = named("s1", stub().await.again, account.path());

        let lines = tokio::time::timeout(CALLING * 3, Entries::of(reader).take())
            .await
            .expect("the call should have been ended at its deadline");

        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].contains("took longer than 10 seconds"),
            "{lines:?}"
        );
    }
}
