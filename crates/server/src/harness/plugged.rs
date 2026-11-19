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
use crate::plugins::manifest::{self, Agent};
use crate::sandbox::Home;
use crate::sandbox::root::{Part, Place};
use crate::settings::AttachedServer;
use crate::store;
use crate::transcript::{Carried, Search};

/// What the path labelled `label` is refused as on a save, a `file` or a
/// directory — see [`Declared::refused`]: in the words of no built-in's, and
/// naming the field the manifest labelled, so that with two paths of one kind
/// the refusal says which.
fn refused(label: &str, file: bool) -> [ProfileSaved; 3] {
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
        .filter(|file| !path.credentials.contains(file) && !path.memory.contains(file))
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
                refused: refused(&path.label, path.file),
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

    fn beside(&self, account: &store::Account) -> Vec<Beside> {
        self.agent.account[1..]
            .iter()
            .filter_map(|path| {
                Some(Beside {
                    host: account.path(&path.key)?.to_owned(),
                    mounted: path.mounted.clone(),
                    file: path.file,
                    credentials: path.credentials.clone(),
                    memory: path.memory.clone(),
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
    fn model(&self, role: store::Role) -> &str {
        self.agent
            .models
            .roles
            .iter()
            .find(|(declared, _)| *declared == role)
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

    fn install(&self) -> Option<Install<'_>> {
        Some(Install {
            instructions: self.agent.install.instructions.as_deref(),
            commands: &self.agent.install.commands,
        })
    }
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
