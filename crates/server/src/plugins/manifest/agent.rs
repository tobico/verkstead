//! A `provides:` entry of the `agent` Category, read and refused by field —
//! the grammar `docs/plugins.md` writes under *An agent*.
//!
//! **Everything here is data** (ADR-0030, *Agent*; Set 1764): which program,
//! which line, which paths an account is, how a session idles, asks and is
//! signed out, which models it offers, and what it is told. The three things
//! about a harness that are code — reading its transcript, writing its
//! configuration and driving its login — are the component's exports, and the
//! manifest only says that it has them.
//!
//! **An entry's own `linux:`, `macos:` and `windows:` replace its keys whole**,
//! as the top-level blocks replace the manifest's: what a harness runs with on
//! a platform is what one block says. Every key of an agent may differ by
//! platform, and every block is checked whichever platform is reading.

use std::path::PathBuf;

use serde_json::{Map, Value};

use super::{
    PLATFORMS, Refusal, Walk, entry, is_word, optional_within, platform_key, relative, text_of,
    within,
};
use crate::platform::Platform;
use crate::store;

/// The keys an agent's entry holds, its `category` and the platform blocks
/// aside.
const KEYS: &[&str] = &[
    "binary",
    "name",
    "mark",
    "launch",
    "account",
    "idle",
    "phrases",
    "ask",
    "models",
    "tools",
    "install",
    "login",
    "environment",
    "instructions",
    "excluded",
];

/// The words a launch line may hold for the host to fill: the model, the
/// Brief, the session id Verkstead named, the Worktree the session works in,
/// and the file the session-only settings are written to.
const PLACEHOLDERS: &[&str] = &["model", "prompt", "session", "worktree", "settings"];

/// A harness, as a plugin's manifest declares it.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    /// The program a session runs, looked for on PATH by this name.
    pub binary: String,
    /// What it is called where a sentence names it.
    pub name: String,
    /// Its own mark, or `None` where the plugin's serves.
    pub mark: Option<super::Mark>,
    pub launch: Launch,
    /// The paths an account of it is made of, in the order the form draws them.
    pub account: Vec<AccountPath>,
    pub idle: Idle,
    /// The stable start of what a session prints when its account is out of
    /// window.
    pub exhausted: Option<String>,
    /// What a session says when its account has signed out.
    pub signed_out: Option<SignedOut>,
    pub ask: Ask,
    pub models: Models,
    /// What the harness puts in front of an MCP server's tool names, with
    /// `<server>` and `<tool>` where the two names go.
    pub tools: String,
    pub install: Install,
    /// How Verkstead runs its login, or `None` for one Verkstead does not log
    /// in.
    pub login: Option<Login>,
    /// What a session is told in its environment beyond what every session is.
    pub environment: Vec<(String, String)>,
    /// The file it reads as its global instructions, said from where the
    /// account lands.
    pub instructions: PathBuf,
    /// What it writes inside the Worktree, each a line of `info/exclude`.
    pub excluded: Vec<String>,
}

/// A launch line, each part a list of words with placeholders in it.
#[derive(Debug, Clone, PartialEq)]
pub struct Launch {
    /// How the model is said, holding `{model}`.
    pub model: Vec<String>,
    /// How the Brief is given, holding `{prompt}`: a flag and it, or it alone.
    pub prompt: Vec<String>,
    /// How the session Verkstead named is named, holding `{session}`, or
    /// `None` for a harness that takes no session id.
    pub session: Option<Vec<String>>,
    /// How a conversation is carried on, or `None` where it is not: under the
    /// id the session was named, or, where `session` is `None`, the one its
    /// record gave.
    pub resume: Option<Resume>,
    /// What goes last.
    pub tail: Vec<String>,
    /// How the Worktree is said, holding `{worktree}`: written after the tail,
    /// and left off whole for a session with no Worktree.
    pub worktree: Vec<String>,
    /// The settings for this session only, written as JSON to the file
    /// `{settings}` names.
    pub settings: Option<Value>,
}

/// How a harness carries a conversation on.
#[derive(Debug, Clone, PartialEq)]
pub enum Resume {
    /// Words, holding `{session}`, written where the session would have been
    /// named.
    Flag(Vec<String>),
    /// Words written straight after the binary, and what else the resume needs
    /// on the end. Where the words hold no `{session}` the id is a positional
    /// in front of the prompt.
    Subcommand {
        words: Vec<String>,
        also: Vec<String>,
    },
}

/// One path an account is made of.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountPath {
    /// What the account holds it under.
    pub key: String,
    /// What the Profile form labels its field.
    pub label: String,
    /// What it is for, said under the field.
    pub purpose: Option<String>,
    /// What the field shows before anything is typed into it.
    pub example: Option<String>,
    pub file: bool,
    /// Where it is mounted in a session's HOME, said from there.
    pub mounted: PathBuf,
    /// The account's configuration files in a directory, each said from it:
    /// what `write-configuration` is handed as found. Nothing else of the
    /// directory reaches a session but its credentials and its memory.
    pub allowlist: Vec<PathBuf>,
    /// The files in it that hold the login.
    pub credentials: Vec<PathBuf>,
    /// The directories in it that are the memory store.
    pub memory: Vec<Memory>,
}

/// One directory of an account's memory store, and what of it crosses to a
/// device the Profile is not bound on.
#[derive(Debug, Clone, PartialEq)]
pub struct Memory {
    pub path: PathBuf,
    /// `None` for memory that stays on the device it was written on.
    pub synced: Option<Synced>,
    /// What it is called on the link between devices, where that isn't its
    /// path — see [`crate::sandbox::root::Part::label`]: the word a device on
    /// an older release still sends it under.
    pub part: Option<String>,
}

/// Which of a memory directory's files a session away from home is given and
/// sends back: one of the host's own rules, named — see
/// [`crate::sandbox::root::Whose`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Synced {
    /// Every file under it.
    Everything,
    /// The rollouts — `rollout-*.jsonl`, opening with a
    /// `session_meta` line — whose session worked in this Worktree.
    Worktree,
    /// The directories in it, or one level down, called by one of the
    /// Conversation's session ids.
    Sessions,
    /// The SQLite database of this name directly in it, with its `-wal` and
    /// `-shm`.
    Database(String),
}

/// The words `synced` may be, in the order a refusal lists them.
const SYNCED: &[&str] = &["everything", "worktree", "sessions", "database"];

/// How a session is told to be idle from at work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Idle {
    /// It is at work while this is on its Screen.
    Signature(String),
    /// It is idle once it has written nothing for a while.
    Silence,
}

/// What a session says when its account has signed out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedOut {
    /// The sentence a line ends with.
    pub ends: String,
    /// The line drawn next, where the sentence alone is also said signed in.
    pub then: Option<String>,
}

/// How a session puts a Question Set to the human.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    pub channel: store::Channel,
    /// The Guide's *Running the ask* section, as markdown.
    pub guide: String,
}

/// The models a Profile of it offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Models {
    pub list: Vec<Model>,
    /// The id of the one a Pairing runs where nothing else says.
    pub default: String,
    /// The roles whose default is another, with that one's id.
    pub roles: Vec<(store::Role, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub id: String,
    pub name: String,
}

/// How the harness's binary is put on the machine.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Install {
    /// Where its own instructions are.
    pub instructions: Option<String>,
    /// The lines that install it, run only on a press.
    pub commands: Vec<String>,
    /// The package npm carries it as, which the wizard installs the way it
    /// installs a built-in's: joined onto the one raised batch on a Linux,
    /// with node in front where the machine has no npm, and raised with
    /// winget's node in front on Windows.
    pub npm: Option<String>,
    /// The Homebrew formula it is, behind the Homebrew the wizard installs
    /// first on a Mac that has none.
    pub brew: Option<String>,
    /// Or the Homebrew cask, the same way.
    pub cask: Option<String>,
    /// And the nixpkgs attribute, which NixOS's by-hand line names.
    pub nix: Option<String>,
}

/// How Verkstead runs a login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login {
    /// The words after the binary that log in.
    pub line: Vec<String>,
    /// And the words that ask whether the account is logged in.
    pub status: Vec<String>,
    /// Whether the harness reads the code off a terminal rather than a pipe.
    pub terminal: bool,
}

impl Walk<'_> {
    /// The body of `field`, an `agent` entry, as `self`'s platform reads it.
    pub(super) fn agent(&self, body: &Map<String, Value>, field: &str) -> Result<Agent, Refusal> {
        let fields = overridden(body, self.platform, field)?;
        let at = |key: &str| format!("{field}.{key}");

        let login = match fields.get("login") {
            Some(Value::Null) | None => None,
            Some(value) => Some(login(value, &at("login"))?),
        };
        let (exhausted, signed_out) = phrases(&fields, field, login.is_some())?;

        Ok(Agent {
            binary: binary(within(&fields, field, "binary")?, &at("binary"))?,
            name: text_of(within(&fields, field, "name")?, &at("name"))?,
            mark: self.mark(&fields, "mark", &at("mark"))?,
            launch: launch(within(&fields, field, "launch")?, &at("launch"))?,
            account: account(within(&fields, field, "account")?, &at("account"))?,
            idle: idle(within(&fields, field, "idle")?, &at("idle"))?,
            exhausted,
            signed_out,
            ask: ask(within(&fields, field, "ask")?, &at("ask"))?,
            models: models(within(&fields, field, "models")?, &at("models"))?,
            tools: tools(within(&fields, field, "tools")?, &at("tools"))?,
            install: install(&fields, &at("install"))?,
            login,
            environment: environment(&fields, &at("environment"))?,
            instructions: relative(
                &text_of(within(&fields, field, "instructions")?, &at("instructions"))?,
                &at("instructions"),
            )?,
            excluded: texts(&fields, "excluded", &at("excluded"))?,
        })
    }
}

/// The program at `field`: a name, looked for on PATH, and never a path.
///
/// **A path would be a grant.** The sandbox follows every harness's program
/// into the directory it is installed in and opens that directory, read-only,
/// to every session — see [`crate::sandbox::installs`] — so a binary named as
/// a file under the home would open the directory holding it to sessions of
/// every harness, which nothing the human agreed to covers.
fn binary(value: &Value, field: &str) -> Result<String, Refusal> {
    let name = text_of(value, field)?;
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\', ':']) {
        return Err(Refusal::of(
            field,
            format!(
                "`{field}` should be the program's name, which is looked for on PATH, rather \
                 than a path."
            ),
        ));
    }
    Ok(name)
}

/// The entry with `platform`'s block written over it, key by key, and every
/// platform's block checked.
fn overridden(
    body: &Map<String, Value>,
    platform: Platform,
    field: &str,
) -> Result<Map<String, Value>, Refusal> {
    let mut fields = body.clone();
    let mut chosen = None;

    for other in PLATFORMS {
        let key = platform_key(other);
        let Some(block) = fields.remove(key) else {
            continue;
        };
        let block_field = format!("{field}.{key}");
        let Value::Object(block) = block else {
            return Err(Refusal::of(
                &block_field,
                format!(
                    "`{block_field}` should be a map of the keys that differ on that platform."
                ),
            ));
        };
        if let Some(name) = block.keys().find(|name| !KEYS.contains(&name.as_str())) {
            return Err(Refusal::unknown(&format!("{block_field}.{name}")));
        }
        if other == platform {
            chosen = Some(block);
        }
    }

    if let Some(name) = fields.keys().find(|name| !KEYS.contains(&name.as_str())) {
        return Err(Refusal::unknown(&format!("{field}.{name}")));
    }
    if let Some(block) = chosen {
        fields.extend(block);
    }
    Ok(fields)
}

/// A list of words at `field`, each of whose placeholders is one the host
/// fills, holding `needs` where it is said.
fn words(value: &Value, field: &str, needs: Option<&str>) -> Result<Vec<String>, Refusal> {
    let Value::Array(entries) = value else {
        return Err(Refusal::of(
            field,
            format!("`{field}` should be a list of the words on the line."),
        ));
    };
    let words = entries
        .iter()
        .enumerate()
        .map(|(index, word)| {
            let word_field = format!("{field}[{index}]");
            let word = text_of(word, &word_field)?;
            placeholders(&word, &word_field)?;
            Ok(word)
        })
        .collect::<Result<Vec<String>, Refusal>>()?;

    if let Some(needs) = needs {
        let placeholder = format!("{{{needs}}}");
        if !words.iter().any(|word| word.contains(&placeholder)) {
            return Err(Refusal::of(
                field,
                format!("`{field}` should say where `{placeholder}` goes."),
            ));
        }
    }
    Ok(words)
}

/// Refused where `word` holds a `{name}` the host does not fill.
///
/// **Only a bare name between braces is read as a placeholder.** Any other
/// brace is the word's own: a harness taking an inline table on its line, as a
/// `-c projects={"{worktree}"={trust_level="trusted"}}` does, writes
/// its braces as they are, and the `{worktree}` inside them is still filled.
fn placeholders(word: &str, field: &str) -> Result<(), Refusal> {
    let mut rest = word;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        let name = &rest[open + 1..open + close];
        let bare = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !bare {
            rest = &rest[open + 1..];
            continue;
        }
        if !PLACEHOLDERS.contains(&name) {
            return Err(Refusal::of(
                field,
                format!(
                    "`{field}` holds `{{{name}}}`, which isn't a placeholder. It's one of \
                     {{model}}, {{prompt}}, {{session}}, {{worktree}}, or {{settings}}."
                ),
            ));
        }
        rest = &rest[open + close + 1..];
    }
    Ok(())
}

fn launch(value: &Value, field: &str) -> Result<Launch, Refusal> {
    let map = entry(
        value,
        field,
        &[
            "model", "prompt", "session", "resume", "tail", "worktree", "settings",
        ],
    )?;
    let at = |key: &str| format!("{field}.{key}");
    let optional = |key: &str, needs: Option<&str>| match map.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => words(value, &at(key), needs).map(Some),
    };

    let tail = optional("tail", None)?.unwrap_or_default();
    let settings = match map.get("settings") {
        Some(Value::Null) | None => None,
        Some(value @ Value::Object(_)) => Some(value.clone()),
        Some(_) => {
            return Err(Refusal::of(
                &at("settings"),
                format!(
                    "`{}` should be a map, written to a JSON file.",
                    at("settings")
                ),
            ));
        }
    };
    let launch = Launch {
        model: words(within(map, field, "model")?, &at("model"), Some("model"))?,
        prompt: words(within(map, field, "prompt")?, &at("prompt"), Some("prompt"))?,
        session: optional("session", Some("session"))?,
        resume: match map.get("resume") {
            Some(Value::Null) | None => None,
            Some(value) => Some(resume(value, &at("resume"))?),
        },
        tail,
        worktree: optional("worktree", Some("worktree"))?.unwrap_or_default(),
        settings,
    };

    // `{settings}` may stand in any part of the line, the resume's among them.
    let mut parts: Vec<&[String]> = vec![
        &launch.model,
        &launch.prompt,
        &launch.tail,
        &launch.worktree,
    ];
    parts.extend(launch.session.as_deref());
    match &launch.resume {
        Some(Resume::Flag(words)) => parts.push(words),
        Some(Resume::Subcommand { words, also }) => parts.extend([&words[..], &also[..]]),
        None => {}
    }
    let names_settings = parts
        .into_iter()
        .any(|words| words.iter().any(|word| word.contains("{settings}")));
    if launch.settings.is_some() && !names_settings {
        return Err(Refusal::of(
            &at("tail"),
            format!(
                "`{}` should say where `{{settings}}` goes, since `{}` is written.",
                at("tail"),
                at("settings")
            ),
        ));
    }
    if names_settings && launch.settings.is_none() {
        return Err(Refusal::missing(&at("settings")));
    }

    Ok(launch)
}

fn resume(value: &Value, field: &str) -> Result<Resume, Refusal> {
    let map = entry(value, field, &["flag", "subcommand", "also"])?;
    let at = |key: &str| format!("{field}.{key}");
    let also = match map.get("also") {
        Some(Value::Null) | None => Vec::new(),
        Some(value) => words(value, &at("also"), None)?,
    };

    match (map.get("flag"), map.get("subcommand")) {
        (Some(flag), None) => {
            if map.contains_key("also") {
                return Err(Refusal::of(
                    &at("also"),
                    format!("`{}` goes with a `subcommand` only.", at("also")),
                ));
            }
            Ok(Resume::Flag(words(flag, &at("flag"), Some("session"))?))
        }
        // A subcommand without `{session}` puts the id in front of the
        // prompt.
        (None, Some(subcommand)) => Ok(Resume::Subcommand {
            words: words(subcommand, &at("subcommand"), None)?,
            also,
        }),
        _ => Err(Refusal::of(
            field,
            format!("`{field}` should be either a `flag` or a `subcommand`."),
        )),
    }
}

fn account(value: &Value, field: &str) -> Result<Vec<AccountPath>, Refusal> {
    let Value::Array(entries) = value else {
        return Err(Refusal::of(
            field,
            format!("`{field}` should be a list of the paths an account is made of."),
        ));
    };
    if entries.is_empty() {
        return Err(Refusal::of(
            field,
            format!("`{field}` is empty. An account is at least one path."),
        ));
    }

    let mut paths: Vec<AccountPath> = Vec::new();
    for (index, value) in entries.iter().enumerate() {
        let entry_field = format!("{field}[{index}]");
        let path = account_path(value, &entry_field)?;
        if paths.iter().any(|seen| seen.key == path.key) {
            let key_field = format!("{entry_field}.key");
            return Err(Refusal::of(
                &key_field,
                format!(
                    "`{key_field}` is `{}`, which an earlier path already uses.",
                    path.key
                ),
            ));
        }
        if index == 0 && path.file {
            let kind_field = format!("{entry_field}.kind");
            return Err(Refusal::of(
                &kind_field,
                format!(
                    "`{kind_field}` should be `directory`. An account is said from its first \
                     path, so that one is a directory."
                ),
            ));
        }
        // The first path's credentials are its login, which is one file: the
        // file a login is said from, and the one a root joins.
        if index == 0 && path.credentials.len() > 1 {
            let second = format!("{entry_field}.credentials[1]");
            return Err(Refusal::of(
                &second,
                format!(
                    "`{second}` is a second file. The first path's credentials are the one file \
                     its login is kept in; a later path may hold more."
                ),
            ));
        }
        // What crosses a link is said from the account's directory, which is
        // the first path: a later path's memory stays where it was written.
        if index > 0
            && let Some(at) = path
                .memory
                .iter()
                .position(|memory| memory.synced.is_some())
        {
            let synced_field = format!("{entry_field}.memory[{at}].synced");
            return Err(Refusal::of(
                &synced_field,
                format!(
                    "`{synced_field}` is on a later path. Only the first path's memory is synced \
                     to another device."
                ),
            ));
        }
        paths.push(path);
    }
    Ok(paths)
}

fn account_path(value: &Value, field: &str) -> Result<AccountPath, Refusal> {
    let map = entry(
        value,
        field,
        &[
            "key",
            "label",
            "purpose",
            "example",
            "kind",
            "mounted",
            "allowlist",
            "credentials",
            "memory",
        ],
    )?;
    let at = |key: &str| format!("{field}.{key}");

    let file = match text_of(within(map, field, "kind")?, &at("kind"))?.as_str() {
        "directory" => false,
        "file" => true,
        _ => {
            return Err(Refusal::of(
                &at("kind"),
                format!("`{}` should be `directory` or `file`.", at("kind")),
            ));
        }
    };
    let inside = |key: &str| -> Result<Vec<PathBuf>, Refusal> {
        let written = texts(map, key, &at(key))?;
        if file && !written.is_empty() {
            return Err(Refusal::of(
                &at(key),
                format!("`{}` is only for a directory.", at(key)),
            ));
        }
        written
            .iter()
            .enumerate()
            .map(|(index, path)| relative(path, &format!("{}[{index}]", at(key))))
            .collect()
    };

    // The allowlist is files, read and handed over whole: one written as a
    // directory, or inside the memory store, is no configuration file.
    let allowlist = inside("allowlist")?;
    let memory = memory(map, &at("memory"), file)?;
    let written = texts(map, "allowlist", &at("allowlist"))?;
    for (index, (path, written)) in allowlist.iter().zip(&written).enumerate() {
        if written.ends_with('/') || memory.iter().any(|memory| path.starts_with(&memory.path)) {
            let entry = format!("{}[{index}]", at("allowlist"));
            return Err(Refusal::of(
                &entry,
                format!(
                    "`{entry}` names a directory. `{}` is the account's configuration files.",
                    at("allowlist")
                ),
            ));
        }
    }

    Ok(AccountPath {
        key: text_of(within(map, field, "key")?, &at("key"))?,
        label: text_of(within(map, field, "label")?, &at("label"))?,
        purpose: optional_within(map, field, "purpose")?,
        example: optional_within(map, field, "example")?,
        file,
        mounted: relative(
            &text_of(within(map, field, "mounted")?, &at("mounted"))?,
            &at("mounted"),
        )?,
        allowlist,
        credentials: inside("credentials")?,
        memory,
    })
}

/// The memory directories at `field`: each a path, which stays on the device
/// it was written on, or a map of its `path` and how it is `synced`.
fn memory(map: &Map<String, Value>, field: &str, file: bool) -> Result<Vec<Memory>, Refusal> {
    let entries = match map.get("memory") {
        Some(Value::Null) | None => return Ok(Vec::new()),
        Some(Value::Array(entries)) => entries,
        Some(_) => return Err(Refusal::of(field, format!("`{field}` should be a list."))),
    };
    if file && !entries.is_empty() {
        return Err(Refusal::of(
            field,
            format!("`{field}` is only for a directory."),
        ));
    }

    entries
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let entry_field = format!("{field}[{index}]");
            if let Value::String(path) = value {
                return Ok(Memory {
                    path: relative(path, &entry_field)?,
                    synced: None,
                    part: None,
                });
            }

            let entry = entry(value, &entry_field, &["path", "synced", "database", "part"])?;
            let at = |key: &str| format!("{entry_field}.{key}");
            let path = relative(
                &text_of(within(entry, &entry_field, "path")?, &at("path"))?,
                &at("path"),
            )?;
            let database = optional_within(entry, &entry_field, "database")?;
            let word = optional_within(entry, &entry_field, "synced")?;
            let part = optional_within(entry, &entry_field, "part")?;

            if let Some(part) = &part
                && (part.is_empty() || part.contains(['/', '\\']) || word.is_none())
            {
                let part_field = at("part");
                return Err(Refusal::of(
                    &part_field,
                    format!(
                        "`{part_field}` should be one word, and goes with `{}` only.",
                        at("synced")
                    ),
                ));
            }

            if let Some(word) = word.as_deref()
                && !SYNCED.contains(&word)
            {
                let synced_field = at("synced");
                return Err(Refusal::of(
                    &synced_field,
                    format!(
                        "`{synced_field}` is `{word}`, which isn't a way memory is synced. \
                         It's one of everything, worktree, sessions, or database."
                    ),
                ));
            }

            let synced = match (word.as_deref(), database) {
                (None, None) => None,
                (Some("database"), Some(name)) => {
                    let name_field = at("database");
                    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".."
                    {
                        return Err(Refusal::of(
                            &name_field,
                            format!(
                                "`{name_field}` should be the database file's name, directly \
                                 in `{}`.",
                                at("path")
                            ),
                        ));
                    }
                    Some(Synced::Database(name))
                }
                (Some("database"), None) => return Err(Refusal::missing(&at("database"))),
                (_, Some(_)) => {
                    let name_field = at("database");
                    return Err(Refusal::of(
                        &name_field,
                        format!(
                            "`{name_field}` goes with `{}: database` only.",
                            at("synced")
                        ),
                    ));
                }
                (Some("worktree"), None) => Some(Synced::Worktree),
                (Some("sessions"), None) => Some(Synced::Sessions),
                (Some(_), None) => Some(Synced::Everything),
            };
            Ok(Memory { path, synced, part })
        })
        .collect()
}

fn idle(value: &Value, field: &str) -> Result<Idle, Refusal> {
    match value {
        Value::String(word) if word == "silence" => Ok(Idle::Silence),
        Value::Object(_) => {
            let map = entry(value, field, &["signature"])?;
            let signature = text_of(
                within(map, field, "signature")?,
                &format!("{field}.signature"),
            )?;
            Ok(Idle::Signature(signature))
        }
        _ => Err(Refusal::of(
            field,
            format!(
                "`{field}` should be `silence`, or a map holding the `signature` drawn at work."
            ),
        )),
    }
}

fn phrases(
    fields: &Map<String, Value>,
    agent: &str,
    logs_in: bool,
) -> Result<(Option<String>, Option<SignedOut>), Refusal> {
    let field = format!("{agent}.phrases");
    let map = match fields.get("phrases") {
        Some(Value::Null) | None => return Ok((None, None)),
        Some(value) => entry(value, &field, &["exhausted", "signed-out"])?,
    };

    let signed_out = match map.get("signed-out") {
        Some(Value::Null) | None => None,
        Some(value) => {
            let signed_field = format!("{field}.signed-out");
            if !logs_in {
                return Err(Refusal::of(
                    &signed_field,
                    format!(
                        "`{signed_field}` needs a `login`, since a session signed out is \
                         logged in again by it."
                    ),
                ));
            }
            let signed = entry(value, &signed_field, &["ends", "then"])?;
            Some(SignedOut {
                ends: text_of(
                    within(signed, &signed_field, "ends")?,
                    &format!("{signed_field}.ends"),
                )?,
                then: optional_within(signed, &signed_field, "then")?,
            })
        }
    };

    Ok((optional_within(map, &field, "exhausted")?, signed_out))
}

fn ask(value: &Value, field: &str) -> Result<Ask, Refusal> {
    let map = entry(value, field, &["channel", "guide"])?;
    let channel_field = format!("{field}.channel");
    let channel = match text_of(within(map, field, "channel")?, &channel_field)?.as_str() {
        "blocking" => store::Channel::Blocking,
        "store-and-nudge" => store::Channel::StoreAndNudge,
        _ => {
            return Err(Refusal::of(
                &channel_field,
                format!("`{channel_field}` should be `blocking` or `store-and-nudge`."),
            ));
        }
    };
    Ok(Ask {
        channel,
        guide: text_of(within(map, field, "guide")?, &format!("{field}.guide"))?,
    })
}

fn models(value: &Value, field: &str) -> Result<Models, Refusal> {
    let Value::Array(entries) = value else {
        return Err(Refusal::of(
            field,
            format!("`{field}` should be a list of the models it offers."),
        ));
    };

    let mut list: Vec<Model> = Vec::new();
    let mut default = None;
    let mut roles: Vec<(store::Role, String)> = Vec::new();

    for (index, value) in entries.iter().enumerate() {
        let entry_field = format!("{field}[{index}]");
        let at = |key: &str| format!("{entry_field}.{key}");
        let map = entry(value, &entry_field, &["id", "name", "default", "for"])?;
        let id = text_of(within(map, &entry_field, "id")?, &at("id"))?;
        if list.iter().any(|seen| seen.id == id) {
            return Err(Refusal::of(
                &at("id"),
                format!(
                    "`{}` is `{id}`, which an earlier model already is.",
                    at("id")
                ),
            ));
        }

        match map.get("default") {
            Some(Value::Null) | None | Some(Value::Bool(false)) => {}
            Some(Value::Bool(true)) if default.is_none() => default = Some(id.clone()),
            Some(Value::Bool(true)) => {
                return Err(Refusal::of(
                    &at("default"),
                    format!(
                        "`{}` is a second default. Only one model is.",
                        at("default")
                    ),
                ));
            }
            Some(_) => {
                return Err(Refusal::of(
                    &at("default"),
                    format!("`{}` should be `true` or `false`.", at("default")),
                ));
            }
        }

        for (role_index, word) in texts(map, "for", &at("for"))?.iter().enumerate() {
            let role_field = format!("{}[{role_index}]", at("for"));
            if !is_word(word) {
                return Err(Refusal::of(
                    &role_field,
                    format!(
                        "`{role_field}` is `{word}`, which isn't a role's name: lowercase \
                         letters, digits and hyphens, like `implementation`."
                    ),
                ));
            }
            let role = store::Role::named(word);
            if roles.iter().any(|(seen, _)| *seen == role) {
                return Err(Refusal::of(
                    &role_field,
                    format!("`{role_field}` is `{word}`, which an earlier model is already for."),
                ));
            }
            roles.push((role, id.clone()));
        }

        list.push(Model {
            name: text_of(within(map, &entry_field, "name")?, &at("name"))?,
            id,
        });
    }

    let Some(default) = default else {
        return Err(Refusal::of(
            field,
            format!("`{field}` should have one model with `default: true`."),
        ));
    };
    Ok(Models {
        list,
        default,
        roles,
    })
}

fn tools(value: &Value, field: &str) -> Result<String, Refusal> {
    let tools = text_of(value, field)?;
    if tools.contains("<server>") && tools.contains("<tool>") {
        Ok(tools)
    } else {
        Err(Refusal::of(
            field,
            format!(
                "`{field}` should say where `<server>` and `<tool>` go, like `mcp__<server>__<tool>`."
            ),
        ))
    }
}

fn install(fields: &Map<String, Value>, field: &str) -> Result<Install, Refusal> {
    let map = match fields.get("install") {
        Some(Value::Null) | None => return Ok(Install::default()),
        Some(value) => entry(
            value,
            field,
            &["instructions", "commands", "npm", "brew", "cask", "nix"],
        )?,
    };
    let instructions = optional_within(map, field, "instructions")?;
    if let Some(link) = &instructions {
        let lower = link.to_ascii_lowercase();
        if !(lower.starts_with("https://") || lower.starts_with("http://")) {
            let link_field = format!("{field}.instructions");
            return Err(Refusal::of(
                &link_field,
                format!("`{link_field}` should be a web address, starting `https://`."),
            ));
        }
    }
    Ok(Install {
        instructions,
        commands: texts(map, "commands", &format!("{field}.commands"))?,
        npm: package(map, field, "npm")?,
        brew: package(map, field, "brew")?,
        cask: package(map, field, "cask")?,
        nix: package(map, field, "nix")?,
    })
}

/// A package's name at `key`, where there is one.
///
/// **One word of a package name's own characters**, because the host writes
/// it into a line it runs — on a Linux, raised — so a space, a `;` or a
/// leading `-` would be a second command or a flag rather than a name.
fn package(map: &Map<String, Value>, field: &str, key: &str) -> Result<Option<String>, Refusal> {
    let Some(name) = optional_within(map, field, key)? else {
        return Ok(None);
    };
    let named = !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@/._+-".contains(c));
    if !named {
        let at = format!("{field}.{key}");
        return Err(Refusal::of(
            &at,
            format!("`{at}` should be one package name, such as `@scope/name`."),
        ));
    }
    Ok(Some(name))
}

fn login(value: &Value, field: &str) -> Result<Login, Refusal> {
    let map = entry(value, field, &["line", "status", "terminal"])?;
    let terminal_field = format!("{field}.terminal");
    let terminal = match map.get("terminal") {
        Some(Value::Null) | None => false,
        Some(Value::Bool(terminal)) => *terminal,
        Some(_) => {
            return Err(Refusal::of(
                &terminal_field,
                format!("`{terminal_field}` should be `true` or `false`."),
            ));
        }
    };
    Ok(Login {
        line: unfilled(within(map, field, "line")?, &format!("{field}.line"))?,
        status: unfilled(within(map, field, "status")?, &format!("{field}.status"))?,
        terminal,
    })
}

/// A list of words at `field` that the host fills nothing into: a login's
/// lines are run as they are written, so a placeholder there would reach the
/// harness as the braces themselves.
fn unfilled(value: &Value, field: &str) -> Result<Vec<String>, Refusal> {
    let words = words(value, field, None)?;
    for (index, word) in words.iter().enumerate() {
        if let Some(placeholder) = PLACEHOLDERS
            .iter()
            .find(|name| word.contains(&format!("{{{name}}}")))
        {
            let word_field = format!("{field}[{index}]");
            return Err(Refusal::of(
                &word_field,
                format!(
                    "`{word_field}` holds `{{{placeholder}}}`, which isn't filled on a login's                      line."
                ),
            ));
        }
    }
    Ok(words)
}

fn environment(fields: &Map<String, Value>, field: &str) -> Result<Vec<(String, String)>, Refusal> {
    match fields.get("environment") {
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(Value::Object(map)) => map
            .iter()
            .map(|(name, value)| {
                let value = match value {
                    Value::String(text) => text.clone(),
                    _ => {
                        let name_field = format!("{field}.{name}");
                        return Err(Refusal::of(
                            &name_field,
                            format!("`{name_field}` should be text. Quote a number."),
                        ));
                    }
                };
                Ok((name.clone(), value))
            })
            .collect(),
        Some(_) => Err(Refusal::of(
            field,
            format!("`{field}` should be a map of each variable's name to its value."),
        )),
    }
}

/// The list of text at `key` of `map`, named `field`. Absent is empty.
fn texts(map: &Map<String, Value>, key: &str, field: &str) -> Result<Vec<String>, Refusal> {
    match map.get(key) {
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(Value::Array(entries)) => entries
            .iter()
            .enumerate()
            .map(|(index, value)| text_of(value, &format!("{field}[{index}]")))
            .collect(),
        Some(_) => Err(Refusal::of(field, format!("`{field}` should be a list."))),
    }
}

#[cfg(test)]
pub(in crate::plugins) mod tests {
    use super::super::{Body, Files, Manifest, Mark};
    use super::*;

    /// A harness declared in full: every required piece, and most of the
    /// optional ones. Its component is `agent.wasm` and its mark `mark.svg`.
    pub(in crate::plugins) const AGENT: &str = r#"
uuid: 3f0c9a12-7b4e-4c1d-8e2f-5a6b7c8d9e0f
name: Stub harness
author:
  name: Ada Lovelace
  email: ada@example.com
version: 0.1.0
interface: 1
mark: mark.svg
component: agent.wasm
provides:
  - category: agent
    binary: stub
    name: Stub
    launch:
      model: [--model, "{model}"]
      prompt: ["{prompt}"]
      session: [--session-id, "{session}"]
      resume:
        flag: [--resume, "{session}"]
      tail: [--yes, --settings, "{settings}", --add-dir, "{worktree}"]
      settings:
        quiet: true
    account:
      - key: dir
        label: Directory
        purpose: Mounted at `~/.stub`.
        example: /home/you/accounts/work/.stub
        kind: directory
        mounted: .stub
        allowlist: [settings.json, config.toml]
        credentials: [auth.json]
        memory: [memory]
      - key: file
        label: Settings file
        kind: file
        mounted: .stub.json
    idle:
      signature: esc to interrupt
    phrases:
      exhausted: you've hit your limit
      signed-out:
        ends: Please log in again.
    ask:
      channel: store-and-nudge
      guide: |
        Run `verkstead ask`, then end your turn.
    models:
      - id: stub-large
        name: Stub Large
        default: true
      - id: stub-small
        name: Stub Small
        for: [review]
    tools: mcp__<server>__<tool>
    install:
      instructions: https://example.com/stub
      commands: [npm install -g stub]
      npm: "@example/stub"
      nix: stub
    login:
      line: [login]
      status: [status]
    environment:
      STUB_NO_UPDATE: "1"
    instructions: STUB.md
    excluded: [.stub/]
"#;

    const FILES: &[(&str, &[u8])] = &[
        ("mark.svg", b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
        ("agent.wasm", b"\0asm"),
    ];

    fn read_for(yaml: &str, platform: Platform) -> Result<Manifest, Refusal> {
        Manifest::parse(yaml, Files::Embedded(FILES), platform)
    }

    fn agent_for(yaml: &str, platform: Platform) -> Agent {
        let manifest = read_for(yaml, platform).expect("the agent should be read");
        let Body::Agent(agent) = &manifest.provides[0].body else {
            panic!("an agent's body is read by its grammar");
        };
        (**agent).clone()
    }

    /// The field a manifest was refused at on Linux, insisting it was refused.
    fn refused_at(yaml: &str) -> String {
        let refusal = read_for(yaml, Platform::Linux).expect_err("the manifest should be refused");
        refusal.field.expect("the refusal should name a field")
    }

    #[test]
    fn a_complete_agent_is_read() {
        let agent = agent_for(AGENT, Platform::Linux);

        assert_eq!(agent.binary, "stub");
        assert_eq!(agent.name, "Stub");
        assert_eq!(agent.mark, None);
        assert_eq!(agent.launch.model, ["--model", "{model}"]);
        assert_eq!(agent.launch.prompt, ["{prompt}"]);
        assert_eq!(
            agent.launch.resume,
            Some(Resume::Flag(vec!["--resume".into(), "{session}".into()]))
        );
        assert_eq!(
            agent.launch.settings,
            Some(serde_json::json!({ "quiet": true }))
        );
        assert_eq!(agent.account.len(), 2);
        assert_eq!(agent.account[0].credentials, [PathBuf::from("auth.json")]);
        assert!(agent.account[1].file);
        assert_eq!(agent.idle, Idle::Signature("esc to interrupt".into()));
        assert_eq!(agent.signed_out.unwrap().ends, "Please log in again.");
        assert_eq!(agent.ask.channel, store::Channel::StoreAndNudge);
        assert_eq!(agent.models.default, "stub-large");
        assert_eq!(
            agent.models.roles,
            [(store::Role::review(), "stub-small".to_owned())]
        );
        assert_eq!(agent.install.commands, ["npm install -g stub"]);
        assert_eq!(agent.install.npm.as_deref(), Some("@example/stub"));
        assert_eq!(agent.install.nix.as_deref(), Some("stub"));
        assert_eq!(agent.install.brew, None);
        assert_eq!(agent.login.unwrap().line, ["login"]);
        assert_eq!(agent.environment, [("STUB_NO_UPDATE".into(), "1".into())]);
        assert_eq!(agent.instructions, PathBuf::from("STUB.md"));
        assert_eq!(agent.excluded, [".stub/"]);
    }

    /// The example `docs/plugins.md` writes under *Agent* is one this
    /// reads, so the grammar and its document say the same.
    #[test]
    fn the_documented_example_is_read() {
        let docs = include_str!("../../../../../docs/plugins.md");
        let (_, after) = docs.split_once("\n### Agent\n").unwrap();
        let (_, after) = after.split_once("```yaml\n").unwrap();
        let (example, _) = after.split_once("```").unwrap();
        let (header, _) = AGENT.split_once("provides:\n").unwrap();
        let yaml = format!("{header}{example}").replace("mark: stub.svg", "mark: mark.svg");

        let agent = agent_for(&yaml, Platform::Linux);

        assert_eq!(agent.binary, "stub");
        assert_eq!(agent.models.default, "stub-large");
    }

    #[test]
    fn a_missing_required_piece_is_refused_naming_it() {
        for (piece, field) in [
            ("    binary: stub\n", "provides[0].binary"),
            ("    tools: mcp__<server>__<tool>\n", "provides[0].tools"),
            ("    instructions: STUB.md\n", "provides[0].instructions"),
            (
                "        label: Settings file\n",
                "provides[0].account[1].label",
            ),
            (
                "      model: [--model, \"{model}\"]\n",
                "provides[0].launch.model",
            ),
        ] {
            assert_eq!(refused_at(&AGENT.replacen(piece, "", 1)), field, "{piece}");
        }
    }

    #[test]
    fn an_unknown_key_in_an_agent_is_refused_naming_it() {
        let at_the_top =
            AGENT.replace("    binary: stub\n", "    binary: stub\n    binery: stub\n");
        let in_a_path = AGENT.replace(
            "        kind: file\n",
            "        kind: file\n        mode: 0600\n",
        );

        assert_eq!(refused_at(&at_the_top), "provides[0].binery");
        assert_eq!(refused_at(&in_a_path), "provides[0].account[1].mode");
    }

    #[test]
    fn a_binary_that_is_a_path_is_refused() {
        for path in [
            "/home/you/.ssh/id_ed25519",
            "../../.ssh/id",
            "bin\\stub.exe",
            "C:stub",
            "..",
        ] {
            let yaml = AGENT.replace("    binary: stub\n", &format!("    binary: {path:?}\n"));

            assert_eq!(refused_at(&yaml), "provides[0].binary", "{path}");
        }
    }

    /// What the setup wizard draws as a link a human presses: a web page, and
    /// nothing a link could run.
    #[test]
    fn install_instructions_that_are_not_a_web_address_are_refused() {
        for link in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "example.com/stub",
        ] {
            let yaml = AGENT.replace(
                "instructions: https://example.com/stub",
                &format!("instructions: {link:?}"),
            );

            assert_eq!(
                refused_at(&yaml),
                "provides[0].install.instructions",
                "{link}"
            );
        }
    }

    /// A package name is written into a line the host runs, raised on a
    /// Linux, so anything that is not one word of a name's own is refused.
    #[test]
    fn an_install_package_that_is_not_one_name_is_refused() {
        for name in [
            "\"stub; rm -rf /\"",
            "\"--global\"",
            "\"two words\"",
            "\"\"",
        ] {
            let yaml = AGENT.replace("npm: \"@example/stub\"", &format!("npm: {name}"));

            assert_eq!(refused_at(&yaml), "provides[0].install.npm", "{name}");
        }
    }

    #[test]
    fn a_second_agent_is_refused() {
        let (_, entry) = AGENT.split_once("provides:\n").unwrap();
        let yaml = format!("{AGENT}{entry}");

        assert_eq!(refused_at(&yaml), "provides[1]");
    }

    #[test]
    fn a_placeholder_on_a_login_s_line_is_refused() {
        let line = AGENT.replace("line: [login]", "line: [login, --settings, \"{settings}\"]");
        let status = AGENT.replace("status: [status]", "status: [status, \"{model}\"]");

        assert_eq!(refused_at(&line), "provides[0].login.line[2]");
        assert_eq!(refused_at(&status), "provides[0].login.status[1]");
    }

    #[test]
    fn a_second_credentials_file_on_the_first_path_is_refused() {
        let yaml = AGENT.replace(
            "credentials: [auth.json]",
            "credentials: [auth.json, refresh.json]",
        );

        assert_eq!(refused_at(&yaml), "provides[0].account[0].credentials[1]");
    }

    #[test]
    fn a_placeholder_the_host_does_not_fill_is_refused() {
        let yaml = AGENT.replace("--add-dir, \"{worktree}\"", "--add-dir, \"{repo}\"");

        assert_eq!(refused_at(&yaml), "provides[0].launch.tail[4]");
    }

    /// Codex's trust table, as stage 04 will declare it: braces of the word's
    /// own around a placeholder the host fills.
    #[test]
    fn braces_that_are_not_a_placeholder_are_the_word_s_own() {
        let word = r#"projects={"{worktree}"={trust_level="trusted"}}"#;
        let yaml = AGENT.replace("--add-dir, \"{worktree}\"", &format!("-c, '{word}'"));

        let tail = agent_for(&yaml, Platform::Linux).launch.tail;

        assert_eq!(tail.last().map(String::as_str), Some(word));
        assert_eq!(
            crate::harness::filled(
                &tail[tail.len() - 1..],
                crate::harness::Filling {
                    worktree: Some("/work/tree"),
                    ..Default::default()
                }
            ),
            [r#"projects={"/work/tree"={trust_level="trusted"}}"#]
        );
    }

    /// The Worktree said apart from the tail, which the host leaves off whole
    /// where a session has none — Codex's trust, a flag and a table.
    #[test]
    fn the_worktree_is_said_apart_where_it_is_written() {
        let word = r#"projects={"{worktree}"={trust_level="trusted"}}"#;
        let yaml = AGENT.replace(
            "      settings:\n        quiet: true\n",
            &format!("      worktree: [-c, '{word}']\n      settings:\n        quiet: true\n"),
        );

        let launch = agent_for(&yaml, Platform::Linux).launch;

        assert_eq!(launch.worktree, ["-c", word]);
        assert_eq!(
            agent_for(AGENT, Platform::Linux).launch.worktree,
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_worktree_part_without_its_placeholder_is_refused() {
        let yaml = AGENT.replace(
            "      settings:\n        quiet: true\n",
            "      worktree: [--trusted]\n      settings:\n        quiet: true\n",
        );

        assert_eq!(refused_at(&yaml), "provides[0].launch.worktree");
    }

    #[test]
    fn a_line_part_without_its_placeholder_is_refused() {
        let yaml = AGENT.replace("prompt: [\"{prompt}\"]", "prompt: [--prompt]");

        assert_eq!(refused_at(&yaml), "provides[0].launch.prompt");
    }

    #[test]
    fn settings_not_named_on_the_line_are_refused() {
        let yaml = AGENT.replace("--settings, \"{settings}\", ", "");

        assert_eq!(refused_at(&yaml), "provides[0].launch.tail");
    }

    #[test]
    fn settings_named_in_any_part_of_the_line_are_read() {
        let yaml = AGENT.replace("--settings, \"{settings}\", ", "").replace(
            "flag: [--resume, \"{session}\"]",
            "subcommand: [resume, \"{session}\"]\n        also: [--settings, \"{settings}\"]",
        );

        assert!(agent_for(&yaml, Platform::Linux).launch.settings.is_some());
    }

    #[test]
    fn settings_named_with_none_written_are_refused() {
        let yaml = AGENT
            .replace("--settings, \"{settings}\", ", "")
            .replace(
                "model: [--model, \"{model}\"]",
                "model: [--model, \"{model}\", --settings, \"{settings}\"]",
            )
            .replace("      settings:\n        quiet: true\n", "");

        assert_eq!(refused_at(&yaml), "provides[0].launch.settings");
    }

    /// A harness told no id at launch names itself, and is resumed by the id
    /// its record gave.
    #[test]
    fn a_resume_without_a_session_is_read() {
        let yaml = AGENT.replace("      session: [--session-id, \"{session}\"]\n", "");

        let launch = agent_for(&yaml, Platform::Linux).launch;

        assert_eq!(launch.session, None);
        assert!(launch.resume.is_some());
    }

    /// Codex's resume: a subcommand the id follows as a positional in front
    /// of the prompt, and words of its own on the end.
    #[test]
    fn a_resume_subcommand_without_a_session_is_read() {
        let yaml = AGENT.replace(
            "flag: [--resume, \"{session}\"]",
            "subcommand: [resume]\n        also: [--last-turn]",
        );

        assert_eq!(
            agent_for(&yaml, Platform::Linux).launch.resume,
            Some(Resume::Subcommand {
                words: vec!["resume".into()],
                also: vec!["--last-turn".into()],
            })
        );
    }

    #[test]
    fn a_resume_flag_without_a_session_is_refused() {
        let yaml = AGENT.replace("flag: [--resume, \"{session}\"]", "flag: [--continue]");

        assert_eq!(refused_at(&yaml), "provides[0].launch.resume.flag");
    }

    #[test]
    fn memory_is_read_with_each_way_it_is_synced() {
        let yaml = AGENT.replace(
            "        memory: [memory]\n",
            "        memory:
          - memory
          - path: memories
            synced: everything
          - path: sessions
            synced: worktree
          - path: conversations
            synced: sessions
          - path: data
            synced: database
            database: store.db
",
        );

        let memory = &agent_for(&yaml, Platform::Linux).account[0].memory;

        assert_eq!(
            memory
                .iter()
                .map(|memory| (memory.path.to_str().unwrap(), memory.synced.clone()))
                .collect::<Vec<_>>(),
            [
                ("memory", None),
                ("memories", Some(Synced::Everything)),
                ("sessions", Some(Synced::Worktree)),
                ("conversations", Some(Synced::Sessions)),
                ("data", Some(Synced::Database("store.db".into()))),
            ]
        );
    }

    /// A synced directory may say what it is called on the link, where that
    /// is not its path: one word, and only beside `synced`.
    #[test]
    fn a_synced_directory_may_name_its_part() {
        let memory = |entry: &str| {
            AGENT.replace(
                "        memory: [memory]\n",
                &format!("        memory:\n          - {entry}\n"),
            )
        };

        let named = agent_for(
            &memory("{path: memories, synced: everything, part: memory}"),
            Platform::Linux,
        );
        assert_eq!(named.account[0].memory[0].part.as_deref(), Some("memory"));

        for entry in [
            "{path: memories, part: memory}",
            "{path: memories, synced: everything, part: a/b}",
            "{path: memories, synced: everything, part: ''}",
        ] {
            assert_eq!(
                refused_at(&memory(entry)),
                "provides[0].account[0].memory[0].part",
                "{entry}"
            );
        }
    }

    #[test]
    fn an_unknown_way_of_syncing_is_refused_naming_it() {
        let memory = |entry: &str| {
            AGENT.replace(
                "        memory: [memory]\n",
                &format!("        memory:\n          - {entry}\n"),
            )
        };

        for (entry, field) in [
            (
                "{path: memory, synced: always}",
                "provides[0].account[0].memory[0].synced",
            ),
            (
                "{path: memory, synced: database}",
                "provides[0].account[0].memory[0].database",
            ),
            (
                "{path: memory, synced: everything, database: store.db}",
                "provides[0].account[0].memory[0].database",
            ),
            (
                "{path: memory, synced: database, database: ../store.db}",
                "provides[0].account[0].memory[0].database",
            ),
            (
                "{path: ../memory, synced: everything}",
                "provides[0].account[0].memory[0].path",
            ),
        ] {
            assert_eq!(refused_at(&memory(entry)), field, "{entry}");
        }
    }

    /// What crosses is said from the account's directory, which is the first
    /// path.
    #[test]
    fn a_later_path_s_memory_synced_is_refused() {
        let yaml = AGENT.replace(
            "        kind: file\n        mounted: .stub.json\n",
            "        kind: directory\n        mounted: .stub-data\n        memory:\n          - {path: notes, synced: everything}\n",
        );

        assert_eq!(refused_at(&yaml), "provides[0].account[1].memory[0].synced");
    }

    #[test]
    fn models_need_one_default() {
        let none = AGENT.replace("        default: true\n", "");
        let two = AGENT.replace(
            "        for: [review]\n",
            "        for: [review]\n        default: true\n",
        );

        assert_eq!(refused_at(&none), "provides[0].models");
        assert_eq!(refused_at(&two), "provides[0].models[1].default");
    }

    #[test]
    fn a_model_for_an_unknown_role_is_refused() {
        let yaml = AGENT.replace("for: [review]", "for: [planning]");

        assert_eq!(refused_at(&yaml), "provides[0].models[1].for[0]");
    }

    #[test]
    fn idle_is_silence_or_a_signature() {
        let silent = AGENT.replace(
            "    idle:\n      signature: esc to interrupt\n",
            "    idle: silence\n",
        );
        let neither = AGENT.replace(
            "    idle:\n      signature: esc to interrupt\n",
            "    idle: never\n",
        );

        assert_eq!(agent_for(&silent, Platform::Linux).idle, Idle::Silence);
        assert_eq!(refused_at(&neither), "provides[0].idle");
    }

    #[test]
    fn a_signed_out_phrase_needs_a_login() {
        let yaml = AGENT.replace(
            "    login:\n      line: [login]\n      status: [status]\n",
            "",
        );

        assert_eq!(refused_at(&yaml), "provides[0].phrases.signed-out");
    }

    #[test]
    fn an_account_whose_first_path_is_a_file_is_refused() {
        let yaml = AGENT.replacen("        kind: directory\n", "        kind: file\n", 1);
        let yaml = yaml.replace(
            "        allowlist: [settings.json, config.toml]\n        credentials: [auth.json]\n        memory: [memory]\n",
            "",
        );

        assert_eq!(refused_at(&yaml), "provides[0].account[0].kind");
    }

    #[test]
    fn a_directory_in_the_allowlist_is_refused() {
        let slashed = AGENT.replace("[settings.json, config.toml]", "[settings.json, sessions/]");
        let memory = AGENT.replace(
            "[settings.json, config.toml]",
            "[settings.json, memory/notes.md]",
        );

        assert_eq!(refused_at(&slashed), "provides[0].account[0].allowlist[1]");
        assert_eq!(refused_at(&memory), "provides[0].account[0].allowlist[1]");
    }

    #[test]
    fn an_account_path_reaching_outside_is_refused() {
        let yaml = AGENT.replace("credentials: [auth.json]", "credentials: [../auth.json]");

        assert_eq!(refused_at(&yaml), "provides[0].account[0].credentials[0]");
    }

    #[test]
    fn an_agent_needs_a_component() {
        let yaml = AGENT.replace("component: agent.wasm\n", "");

        assert_eq!(refused_at(&yaml), "component");
    }

    #[test]
    fn an_agent_needs_a_mark_its_own_or_the_plugins() {
        let none = AGENT.replace("mark: mark.svg\n", "");
        let its_own = none.replace("    name: Stub\n", "    name: Stub\n    mark: mark.svg\n");

        assert_eq!(refused_at(&none), "provides[0].mark");
        assert_eq!(
            agent_for(&its_own, Platform::Linux).mark,
            Some(Mark::File(PathBuf::from("mark.svg")))
        );
    }

    #[test]
    fn a_platform_block_replaces_its_key_there_and_nowhere_else() {
        let yaml = format!(
            "{AGENT}    windows:
      binary: stub.exe
      environment:
        STUB_HOME: C:\\stub
"
        );

        let windows = agent_for(&yaml, Platform::Windows);
        let linux = agent_for(&yaml, Platform::Linux);
        let macos = agent_for(&yaml, Platform::MacOs);

        assert_eq!(windows.binary, "stub.exe");
        assert_eq!(
            windows.environment,
            [("STUB_HOME".into(), "C:\\stub".into())]
        );
        assert_eq!(windows.name, "Stub");
        for other in [linux, macos] {
            assert_eq!(other.binary, "stub");
            assert_eq!(other.environment, [("STUB_NO_UPDATE".into(), "1".into())]);
        }
    }

    #[test]
    fn a_platform_block_is_checked_on_every_platform() {
        let yaml = format!("{AGENT}    macos:\n      binery: stub\n");

        for platform in PLATFORMS {
            assert_eq!(
                read_for(&yaml, platform).unwrap_err().field.as_deref(),
                Some("provides[0].macos.binery"),
                "{platform:?}"
            );
        }
    }

    #[test]
    fn a_platform_block_that_breaks_its_platform_is_refused_there_alone() {
        let yaml = format!("{AGENT}    windows:\n      models: []\n");

        assert_eq!(
            read_for(&yaml, Platform::Windows)
                .unwrap_err()
                .field
                .as_deref(),
            Some("provides[0].models")
        );
        assert!(read_for(&yaml, Platform::Linux).is_ok());
    }
}
