//! A built root: the `.claude`, the `.codex` or the `.grok` a session is given in
//! place of the account's whole one — or the two XDG directories an OpenCode
//! session is given in place of its account's.
//!
//! **An allowlist, and nothing outside it.** What a session needs of its account
//! is its login and the store its memory and transcripts are kept in.
//! Everything else in the account's directory is the human's own way of
//! working: plugins, hooks, rules, skills, a global instructions file, the
//! history, and every other repository's transcripts. None of that is a
//! session's, so none of it is in the root, and whatever a harness adds next is
//! absent from it too without anybody having to notice.
//!
//! **One shape for every harness**, in four parts. Which files and directories
//! each part is, is the harness's own — see [`Harness`]:
//!
//! - **The login, linked**, where the account has one.
//! - **The memory store, joined only where the Profile's memory switch is on.**
//!   Off, each of its directories is the root's own and starts empty, so the
//!   session has fresh memory and none of the human's transcripts, and its own
//!   transcript is written into the root — see [`Root::remembering`].
//! - **A configuration file Verkstead writes**, carrying only what the
//!   account's own says about reaching a model — see [`Root::written`].
//! - **The settings page's instructions text, written as the file that
//!   harness reads as its global instructions** — see [`Root::instructed`].
//!   The account's own such file is left out with the rest of how the human
//!   works, and this is what stands in its place. A text nobody typed is no
//!   file.
//!
//! **The root is Verkstead's own directory**, under the Conversation's profile
//! in the Data Directory, emptied and made again as each session starts — see
//! [`super::Access::Built`]. What goes into it is joined rather than copied, so
//! a login from inside and a memory written inside both land in the account:
//! a bind on Linux, a symlink on a Mac, and on Windows a hard link for the login
//! and a junction for each directory. The one exception is a Grok Build login on
//! Linux, which is copied and merged back — see [`Root::login_copied`].
//!
//! **Every path a root says is said from HOME**, which is where it is put. The
//! account's own path is the same one with the account's directory in place of
//! HOME's `.claude`, `.codex` or `.grok`. An OpenCode account is a whole home,
//! so its paths are the same on both sides — see [`Root::built`].
//!
//! **Beside a Claude root, `.claude.json` is copied rather than joined**, so the
//! trust seeded into it is not written straight into the account, and what a
//! session changes in it is merged back as it ends — see [`merged_back`]. Codex
//! has no such file: its Worktree trust is said on the launch line (ADR-0011).
//! Nor has Grok Build.

use std::io;
use std::path::{Path, PathBuf};

use crate::platform::Platform;

/// The file Claude keeps a login in, inside `~/.claude`.
///
/// Not there at all on a Mac whose login is in the Keychain, which is a root
/// with no credentials in it — see [`Root::joined`].
const CREDENTIALS: &str = ".credentials.json";

/// The directory of per-path entries Claude keeps memory and transcripts in.
const PROJECTS: &str = "projects";

/// The settings file Claude reads for the user, inside `~/.claude`.
const SETTINGS: &str = "settings.json";

/// The file Claude reads as its global instructions, inside `~/.claude`.
const CLAUDE_INSTRUCTIONS: &str = "CLAUDE.md";

/// The file the other three read as their global instructions: Codex's and
/// Grok Build's inside the account's own directory, and OpenCode's inside its
/// config directory.
const AGENTS: &str = "AGENTS.md";

/// Of the account's own settings, the keys a root's settings carry over.
///
/// **An allowlist rather than a denylist**, because a denylist drifts every time
/// Claude adds a key. These two are how an API-key login reaches the model:
/// `apiKeyHelper` is the command that prints the key, and `env` is where an
/// account keeps `ANTHROPIC_API_KEY` and the variables beside it. Everything
/// else is how the human works — `hooks`, `enabledPlugins`, `permissions`,
/// `statusLine` — and none of it is a session's.
const CARRIED: [&str; 2] = ["apiKeyHelper", "env"];

/// The file Codex keeps a login in, inside `~/.codex`.
///
/// **Written in place**, which is what makes a link of it the account's file
/// for a session's whole life: read off codex 0.154, whose `codex login` over
/// an `auth.json` already there leaves the same inode behind. The launch line
/// says `cli_auth_credentials_store=file`, so this file is where the login is.
const AUTH: &str = "auth.json";

/// The configuration file Codex reads for the user, inside `~/.codex`.
const CODEX_CONFIG: &str = "config.toml";

/// What Codex and Grok Build both call the directory their sessions write their
/// logs under, inside the account's directory.
const SESSIONS: &str = "sessions";

/// The configuration file Grok Build reads for the user, inside `~/.grok`.
const GROK_CONFIG: &str = "config.toml";

/// The two directories Codex's memory is kept in, inside `~/.codex`: the
/// rollouts every session writes, and the memory files — `MEMORY.md` and
/// `memory_summary.md` — its memories feature writes and reads.
///
/// **And not the rest of what is beside them.** `archived_sessions/` is the
/// human's, and so is every SQLite database at the top of `~/.codex` —
/// `state_5.sqlite`, `memories_1.sqlite`, `thread_history_1.sqlite`,
/// `logs_2.sqlite`, `goals_1.sqlite`, `queue_1.sqlite` in codex 0.154. A
/// database linked a file at a time loses its write-ahead-log siblings and
/// will not open, so each is the session's own and starts empty.
///
/// **A fresh state database beside shared `sessions/` does no paid work**,
/// checked on codex 0.154. Codex fills a fresh `state_5.sqlite` from the
/// rollouts under `sessions/` as it starts (its `backfill_state`), which reads
/// files and calls no model. What would summarise those rollouts is the
/// memories feature's two phases, and `codex features list` says `memories` is
/// off unless `[features]` in `config.toml` turns it on — which the file a root
/// is written never carries, see [`CODEX_CARRIED`].
const CODEX_MEMORY: [&str; 2] = [SESSIONS, "memories"];

/// Of the account's own `config.toml`, the keys a root's carries over.
///
/// **An allowlist**, for [`CARRIED`]'s reason. These two are how an account on
/// a provider of its own reaches the model: `model_provider` names it, and
/// `model_providers` says where it is. Everything else is how the human works —
/// `mcp_servers`, `profiles`, `projects`, hooks, `notify`, `[features]` — and
/// none of it is a session's.
const CODEX_CARRIED: [&str; 2] = ["model_provider", "model_providers"];

/// The two directories Grok Build's memory is kept in, inside `~/.grok`: the
/// sessions every run writes, and the memory files its memory feature writes
/// and searches.
///
/// **Directories rather than files, which is what grok 1.0.13 keeps.** Read off
/// a real run: `memory/` holds a global `MEMORY.md` and a directory per
/// repository, each with a `MEMORY.md` of its own and an `index.sqlite` beside
/// it; `sessions/` holds a directory per working directory, the session logs
/// inside them, and a `session_search.sqlite` at the top.
///
/// **Both databases are in write-ahead-log mode**: grok picks the mode by the
/// filesystem, and on a local disk a run leaves `index.sqlite-wal` and
/// `index.sqlite-shm` beside the file, and `session_search.sqlite` says version
/// 2 in its header. A database joined a file at a time would lose those
/// siblings and not open. Each is inside a directory joined whole, so it comes
/// with its siblings and is never linked on its own.
///
/// Not `skills/`, `plugins/`, `agents/`, `workflows/`, `logs/`, `docs/` or
/// `pager.toml`, which are the human's or grok's own furniture.
const GROK_MEMORY: [&str; 2] = [SESSIONS, "memory"];

/// Of the account's own `config.toml`, what a Grok root's carries over: each a
/// top-level key, or a `table.key` inside one.
///
/// **An allowlist**, for [`CARRIED`]'s reason. Grok has no one key that names a
/// provider, so these are the ones its configuration reference gives to
/// reaching a model and signing in to one: `model` holds custom model
/// definitions and overrides, with their endpoints and keys; `model_providers`
/// names providers a model points at; `auth_provider` names the credential
/// helpers a model mints its token with. `auth`, and `grok_com_config` which is
/// the same table under another name, say how the account signs in — an
/// enterprise identity provider or an external auth command — without which a
/// login from one cannot be refreshed. And of `endpoints`, only where models
/// are listed and reached; the rest of that table is feedback and trace upload.
///
/// Everything else is how the human works — `mcp_servers`, `hooks`, `skills`,
/// `plugins`, `permission`, `memory`, `ui` — and none of it is a session's.
const GROK_CARRIED: [&str; 7] = [
    "model",
    "model_providers",
    "auth_provider",
    "auth",
    "grok_com_config",
    "endpoints.models_base_url",
    "endpoints.models_list_url",
];

/// The directory OpenCode reads its configuration from, inside its home: built
/// either way, and holding nothing but the file Verkstead writes.
///
/// Not the account's `node_modules/`, `package.json`, agents, commands, themes
/// or skills, which are how the human works.
const OPENCODE_CONFIG: &str = super::OPENCODE_CONFIG_INSIDE_HOME;

/// The directory OpenCode keeps its login and its database in, inside its home.
///
/// **Joined whole where memory is shared, and never a file at a time.** Read
/// off opencode 1.18.30: `opencode.db` runs in write-ahead-log mode, with
/// `opencode.db-wal` and `opencode.db-shm` beside it, and a database linked
/// apart from those will not open. The login, `mcp-auth.json`, snapshots and
/// logs travel with it. Where memory is not shared, the directory is the
/// root's own, the database in it starts empty, and the login alone is linked
/// into it.
const OPENCODE_DATA: &str = super::OPENCODE_DATA_INSIDE_HOME;

/// The file OpenCode keeps a login in, inside its home.
///
/// **Written in place**, like Codex's: read off opencode 1.18.30, whose
/// `opencode auth logout` leaves the same inode behind. So a link of it stays
/// the account's file for the session's whole life.
const OPENCODE_AUTH: &str = ".local/share/opencode/auth.json";

/// The configuration files OpenCode reads for the user, inside its config
/// directory, in the order it reads them: a later file's keys win.
const OPENCODE_CONFIGS: [&str; 2] = ["opencode.json", "opencode.jsonc"];

/// Of the account's own configuration, the key a root's carries over:
/// `provider`, which says where a custom provider is and how it is reached.
///
/// **An allowlist**, for [`CARRIED`]'s reason. Everything else is how the human
/// works — `mcp`, `plugin`, `agent`, `command`, `instructions`, `theme` — and
/// none of it is a session's.
const OPENCODE_CARRIED: &str = "provider";

/// Of the account's `.claude.json`, the key its MCP servers are under: at the top
/// level, and again under each `projects` entry.
///
/// **Never in a session's copy, and never written back.** Those are the human's
/// own servers, the same leak as plugins — and a key a copy never had is not a
/// key a session removed.
const MCP_SERVERS: &str = "mcpServers";

/// Of the account's `.claude.json`, the key its per-path entries are under.
const PROJECTS_CONFIG: &str = "projects";

/// What a `projects` entry says to have the trust dialog answered.
const TRUSTED: &str = "hasTrustDialogAccepted";

/// How long an entry's name is before Claude cuts it and puts a hash on the end.
const LONGEST: usize = 200;

/// What a session is given of its account: the account's own directory, which
/// harness's root it is, and whether its memory is shared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Root {
    /// The account's own directory, which the Profile names: `~/.claude`,
    /// `~/.codex`, `~/.grok`, or the home an OpenCode account is kept in.
    account: PathBuf,

    /// Which harness's allowlist this is.
    harness: Harness,

    /// Whether the memory store is joined at all: the Profile's memory switch —
    /// see [`Root::remembering`].
    memory: bool,
}

/// Which harness a root is for, and what that harness's allowlist is made of.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Harness {
    /// Claude Code: `.credentials.json` linked, two entries under `projects/`
    /// as its memory, and a `settings.json` written.
    Claude {
        /// The `projects/` entries joined, in the order they are said: the
        /// Repo's main checkout's first, which holds Claude's per-Repo memory,
        /// and the Worktree's after it, where the session's transcript is
        /// written. One where the two are the same name.
        entries: Vec<String>,

        /// The same two paths as `.claude.json` keys its `projects` entries:
        /// the plain path, with forward slashes on Windows. One where the two
        /// are the same path.
        trusted: Vec<String>,
    },

    /// Codex: `auth.json` linked, `sessions/` and `memories/` as its memory,
    /// and a `config.toml` written — see [`CODEX_MEMORY`].
    Codex,

    /// Grok Build: `auth.json` linked, `sessions/` and `memory/` as its memory,
    /// and a `config.toml` written — see [`GROK_MEMORY`].
    Grok,

    /// OpenCode: its data directory as its memory, joined whole, or built with
    /// `auth.json` alone linked into it — see [`OPENCODE_DATA`]. And its config
    /// directory built, with an `opencode.json` written.
    OpenCode,
}

impl Root {
    /// The Claude root for a session in `worktree`, whose Repo's common git
    /// directory is `git_dir`, logged in as the account at `account`.
    ///
    /// **Each path as Claude will read it inside**, because the name is the
    /// path's. The main checkout is what Claude reads back out of the
    /// Worktree's own `.git` file, which git writes resolved — the same answer
    /// git gave for `git_dir`. The Worktree is the directory Claude was started
    /// in, as the machine reports it: resolved on a Mac and on Windows, where a
    /// session starts in the host's own path, and as stored on Linux, where a
    /// bind makes the Worktree at the path it was stored under and nothing on
    /// the way to it is a link.
    ///
    /// **And plain**, because resolving a path on Windows writes `\\?\` in
    /// front of it, and Claude names an entry from the path it was started in,
    /// which has no such prefix — see [`super::plainly`].
    pub(crate) fn claude(
        platform: Platform,
        account: &Path,
        git_dir: &Path,
        worktree: &Path,
    ) -> Root {
        let worktree = match platform {
            Platform::Linux => worktree.to_owned(),
            Platform::MacOs | Platform::Windows => resolved(worktree),
        };

        let mut entries = Vec::new();
        let mut trusted = Vec::new();

        for path in [main_checkout(git_dir), worktree] {
            let plain = super::plainly(&path);
            let entry = entry_named(&plain);

            if !entries.contains(&entry) {
                entries.push(entry);
            }

            let key = match platform {
                Platform::Windows => plain.to_string_lossy().replace('\\', "/"),
                Platform::Linux | Platform::MacOs => plain.to_string_lossy().into_owned(),
            };

            if !trusted.contains(&key) {
                trusted.push(key);
            }
        }

        Root {
            account: account.to_owned(),
            harness: Harness::Claude { entries, trusted },
            memory: true,
        }
    }

    /// The Codex root for a session logged in as the account at `account`.
    ///
    /// Nothing about the Worktree is in it: Codex keeps one `sessions/` for
    /// every directory it runs in, and its trust is said on the launch line.
    pub(crate) fn codex(account: &Path) -> Root {
        Root {
            account: account.to_owned(),
            harness: Harness::Codex,
            memory: true,
        }
    }

    /// The Grok Build root for a session logged in as the account at `account`.
    ///
    /// Nothing about the Worktree is in it, for Codex's reason: grok keeps one
    /// `sessions/` and one `memory/` for every directory it runs in.
    pub(crate) fn grok(account: &Path) -> Root {
        Root {
            account: account.to_owned(),
            harness: Harness::Grok,
            memory: true,
        }
    }

    /// The OpenCode root for a session logged in as the account whose home is
    /// `account`.
    ///
    /// Nothing about the Worktree is in it, for Codex's reason: opencode keeps
    /// one database for every directory it runs in.
    pub(crate) fn opencode(account: &Path) -> Root {
        Root {
            account: account.to_owned(),
            harness: Harness::OpenCode,
            memory: true,
        }
    }

    /// The same root, sharing the account's memory or not.
    ///
    /// **On**, which is what every constructor above makes: the
    /// memory store is made in the account and joined, so memory a session
    /// writes is the account's and its transcript is in the account's store.
    ///
    /// **Off**: nothing of the account's store is made or joined. Each of its
    /// directories is a directory of the root's own, empty as the session
    /// starts, and the harness writes the session's memory and transcript there
    /// — see [`Root::unshared_in`]. The login and the written configuration are
    /// the same either way.
    pub(crate) fn remembering(self, memory: bool) -> Root {
        Root { memory, ..self }
    }

    /// What this root is called among a Conversation's roots, which is what a
    /// launch into it is registered under — see [`super::sharing`].
    pub(crate) fn named(&self) -> &'static str {
        match self.harness {
            Harness::Claude { .. } => super::CLAUDE_DIR_INSIDE_HOME,
            Harness::Codex => super::CODEX_INSIDE_HOME,
            Harness::Grok => super::GROK_INSIDE_HOME,
            Harness::OpenCode => "opencode",
        }
    }

    /// Where the account's directory lands inside HOME, which is what every
    /// path of the account's is under there: `.claude`, `.codex` or `.grok`,
    /// and HOME itself for OpenCode, whose account is a home.
    fn landing(&self) -> &'static Path {
        Path::new(match self.harness {
            Harness::Claude { .. } | Harness::Codex | Harness::Grok => self.named(),
            Harness::OpenCode => "",
        })
    }

    /// The directories of Verkstead's own this root is made of, each said from
    /// HOME. Each is emptied and made on the host as a session starts, in the
    /// Conversation's own directory, and is put at the same path inside.
    ///
    /// One for Claude, Codex and Grok Build: the account's directory. Two for
    /// OpenCode where memory is not shared: its config directory and its data
    /// directory. And the config directory alone where memory is shared, the
    /// data directory then being the account's, joined whole.
    pub(crate) fn built(&self) -> Vec<&'static str> {
        match (&self.harness, self.memory) {
            (Harness::Claude { .. } | Harness::Codex | Harness::Grok, _) => vec![self.named()],
            (Harness::OpenCode, true) => vec![OPENCODE_CONFIG],
            (Harness::OpenCode, false) => vec![OPENCODE_CONFIG, OPENCODE_DATA],
        }
    }

    /// Where the directory of `projects/` entries is in a Claude root at `root`.
    pub(crate) fn projects_in(root: &Path) -> PathBuf {
        root.join(PROJECTS)
    }

    /// Where a Codex root at `root` keeps its rollouts, and a Grok Build root its
    /// sessions.
    pub(crate) fn sessions_in(root: &Path) -> PathBuf {
        root.join(SESSIONS)
    }

    /// The account's own directory.
    pub(crate) fn account(&self) -> &Path {
        &self.account
    }

    /// The account's login file, whether or not there is one.
    pub(crate) fn credentials(&self) -> PathBuf {
        self.account.join(self.login())
    }

    /// Where the login file is in this root, under the HOME at `home`: the
    /// Conversation's own directory on the host, or HOME inside.
    pub(crate) fn credentials_in(&self, home: &Path) -> PathBuf {
        home.join(self.landing()).join(self.login())
    }

    /// The login file `account` keeps, where a session is given it as a file
    /// of its own, whether or not the file is there. `memory` is the Profile's
    /// memory switch.
    ///
    /// `None` for an OpenCode account sharing its memory, whose login is inside
    /// the data directory joined whole — see [`Root::login_alone`].
    pub(crate) fn login_of(account: &crate::store::Account, memory: bool) -> Option<PathBuf> {
        match account {
            crate::store::Account::Claude { claude_dir, .. } => Some(claude_dir.join(CREDENTIALS)),
            crate::store::Account::Codex { home } | crate::store::Account::Grok { home } => {
                Some(home.join(AUTH))
            }
            crate::store::Account::OpenCode { home } => (!memory).then(|| home.join(OPENCODE_AUTH)),
        }
    }

    /// Whether the login is joined as a file of its own. It always is, except
    /// in an OpenCode root sharing its memory: there the login is inside the
    /// data directory joined whole, so it is the account's with nothing more
    /// said about it and nothing to hand back.
    pub(crate) fn login_alone(&self) -> bool {
        !matches!((&self.harness, self.memory), (Harness::OpenCode, true))
    }

    /// Where the login file is, inside the account's directory.
    fn login(&self) -> &'static str {
        match self.harness {
            Harness::Claude { .. } => CREDENTIALS,
            Harness::Codex | Harness::Grok => AUTH,
            Harness::OpenCode => OPENCODE_AUTH,
        }
    }

    /// The configuration file a root is given: where it goes in a root built in
    /// the Conversation's own directory at `built`, and what it holds. It is
    /// Verkstead's own, written as each session starts, and neither joined nor
    /// written back.
    ///
    /// Read off the account's own file as it is at this moment, so a key the
    /// human changes reaches the next session. Claude's is a `settings.json` —
    /// see [`settings`]. Codex's and Grok Build's is a `config.toml` — see
    /// [`toml_carrying`]. OpenCode's is an `opencode.json` — see
    /// [`opencode_config`].
    ///
    /// Blocking: one read, or two for OpenCode.
    pub(crate) fn written(&self, built: &Path) -> (PathBuf, Vec<u8>) {
        let root = built.join(self.landing());

        match self.harness {
            Harness::Claude { .. } => (
                root.join(SETTINGS),
                settings(std::fs::read(self.account.join(SETTINGS)).ok().as_deref()),
            ),
            Harness::Codex => (
                root.join(CODEX_CONFIG),
                toml_carrying(
                    std::fs::read_to_string(self.account.join(CODEX_CONFIG))
                        .ok()
                        .as_deref(),
                    &CODEX_CARRIED,
                ),
            ),
            Harness::Grok => (
                root.join(GROK_CONFIG),
                toml_carrying(
                    std::fs::read_to_string(self.account.join(GROK_CONFIG))
                        .ok()
                        .as_deref(),
                    &GROK_CARRIED,
                ),
            ),
            Harness::OpenCode => {
                let config = Path::new(OPENCODE_CONFIG);

                (
                    root.join(config).join(OPENCODE_CONFIGS[0]),
                    opencode_config(OPENCODE_CONFIGS.map(|file| {
                        std::fs::read_to_string(self.account.join(config).join(file)).ok()
                    })),
                )
            }
        }
    }

    /// The global instructions file a root is given: where it goes in a root
    /// built in the Conversation's own directory at `built`, and what it
    /// holds. `None` where there is no text to give.
    ///
    /// **The settings page's one text, verbatim.** No heading over it and no
    /// line saying where it came from: what the human typed is what the
    /// harness reads. It stands where the account's own global instructions
    /// file would have been, which a root carries none of — see this module's
    /// allowlist.
    ///
    /// Each harness names its own, and each is inside a directory the root
    /// already builds: `.claude/CLAUDE.md` for Claude, `.codex/AGENTS.md` for
    /// Codex, `.grok/AGENTS.md` for Grok Build, and the config directory's
    /// `AGENTS.md` for OpenCode.
    ///
    /// **Written rather than joined**, exactly as the configuration file
    /// beside it is: it is Verkstead's own file, it is never the account's,
    /// and nothing of it is written back. Read off the settings at the moment
    /// the root is built, so a text saved on the settings page reaches the
    /// next session and a running one keeps what it started with.
    ///
    /// **A text of nothing is no file at all**, so a root then holds what it
    /// held before there was such a setting. Which text is nothing is the
    /// settings file's own answer — see [`crate::settings::Config`], where a
    /// text of only whitespace has already become none.
    ///
    /// The Repo's own `CLAUDE.md` or `AGENTS.md` is in the Worktree, is the
    /// Repo's, and is untouched by this: each harness reads both, this one
    /// above it.
    pub(crate) fn instructed(&self, built: &Path, text: &str) -> Option<(PathBuf, Vec<u8>)> {
        if text.is_empty() {
            return None;
        }

        let root = built.join(self.landing());

        let path = match self.harness {
            Harness::Claude { .. } => root.join(CLAUDE_INSTRUCTIONS),
            Harness::Codex | Harness::Grok => root.join(AGENTS),
            Harness::OpenCode => root.join(OPENCODE_CONFIG).join(AGENTS),
        };

        Some((path, text.as_bytes().to_vec()))
    }

    /// Whether a session on `platform` is given a copy of the login rather
    /// than a link to it: a Grok Build root on Linux, and no other.
    ///
    /// **Grok saves its login by renaming a file over it, and nothing else.**
    /// Read off grok 1.0.13: a save writes a temporary file beside `auth.json`
    /// and renames it over the top, and falls back to writing in place only
    /// when the disk is full. A bind refuses the rename — `grok logout` inside
    /// bubblewrap over a bound `auth.json` fails with *Resource busy* — so a
    /// session given one could never save a login or a refreshed token. Claude
    /// falls back to writing in place whenever the rename is refused, and Codex
    /// writes in place to begin with, so a bind serves both of them.
    ///
    /// **A copy, merged back as the session ends**, the way Claude's
    /// `.claude.json` is — see [`merged_back`]. The top-level keys of Grok's
    /// `auth.json` are its login scopes, so a scope the session refreshed goes
    /// back and one it did not touch stays as the account has it, whatever the
    /// human's own `grok` wrote there meanwhile. On a Mac and on Windows the
    /// rename replaces the link instead, which is handed back as it ends — see
    /// [`super::closing`].
    pub(crate) fn login_copied(&self, platform: Platform) -> bool {
        matches!((&self.harness, platform), (Harness::Grok, Platform::Linux))
    }

    /// The `.claude.json` a Claude session is given: a copy of the account's own
    /// at `config_file` as it is at this moment, with its MCP servers taken out
    /// and the Repo and the Worktree trusted — see [`config`].
    ///
    /// Copied rather than linked, so what is seeded is written into the copy
    /// and not into the account. What the session changes goes back as it ends
    /// — see [`merged_back`].
    ///
    /// Blocking: one read.
    pub(crate) fn config(&self, config_file: &Path) -> Vec<u8> {
        let trusted = match &self.harness {
            Harness::Claude { trusted, .. } => trusted.as_slice(),
            Harness::Codex | Harness::Grok | Harness::OpenCode => &[],
        };

        config(std::fs::read(config_file).ok().as_deref(), trusted)
    }

    /// Make each directory of the memory store in the account where it is not
    /// there yet.
    ///
    /// **Made in the account rather than in the root**, because what is written
    /// under one is the account's: memory a later session reads, and the
    /// transcript Verkstead follows. A directory that is not there is the
    /// ordinary case — a Repo nobody has run Claude in yet, a Codex account
    /// that has never written a memory — and a join of nothing is a session
    /// that will not start.
    ///
    /// **Nothing at all where memory is off**: nothing is joined, so nothing of
    /// the account's is written to make a join of.
    ///
    /// Blocking.
    pub(crate) fn made_in_account(&self) -> io::Result<()> {
        for store in self.joined_store() {
            std::fs::create_dir_all(self.account.join(store))?;
        }

        Ok(())
    }

    /// Everything joined into a root a session finds in the HOME at `inside`:
    /// each as the account's own path and the path a session finds it at.
    ///
    /// The login first, and only where the account has one. A join of a file
    /// that is not there is nothing on a Mac, a hard link that fails on Windows
    /// and a bind that will not start on Linux — so a root for an account with
    /// no file has none, and the file a session logs in and writes is handed
    /// back as it ends instead. See [`super::Sandbox::command`].
    ///
    /// Blocking: one `stat`.
    pub(crate) fn joined(&self, inside: &Path) -> Vec<(PathBuf, PathBuf)> {
        let mut joined = Vec::new();

        let credentials = self.credentials();

        if self.login_alone() && credentials.is_file() {
            joined.push((credentials, self.credentials_in(inside)));
        }

        for store in self.joined_store() {
            joined.push((
                self.account.join(&store),
                inside.join(self.landing()).join(&store),
            ));
        }

        joined
    }

    /// The directories inside a root built in the Conversation's own directory
    /// at `built` that are its own and made empty as it is built: the memory
    /// store's, where memory is not shared, and none where it is.
    ///
    /// Claude's is the whole of `projects/`, the entries being named inside it
    /// as the session writes them. Codex's are `sessions/` and `memories/`, and
    /// Grok Build's `sessions/` and `memory/`. OpenCode's is none: its data
    /// directory is then one of the root's own directories already — see
    /// [`Root::built`].
    pub(crate) fn unshared_in(&self, built: &Path) -> Vec<PathBuf> {
        if self.memory {
            return Vec::new();
        }

        let root = built.join(self.landing());

        match self.harness {
            Harness::Claude { .. } => vec![Root::projects_in(&root)],
            Harness::Codex => CODEX_MEMORY.iter().map(|store| root.join(store)).collect(),
            Harness::Grok => GROK_MEMORY.iter().map(|store| root.join(store)).collect(),
            Harness::OpenCode => Vec::new(),
        }
    }

    /// The memory store joined from the account, each as a path relative to
    /// the account's directory: all of it where memory is shared, and none of
    /// it where it is not.
    fn joined_store(&self) -> Vec<PathBuf> {
        if !self.memory {
            return Vec::new();
        }

        match &self.harness {
            Harness::Claude { entries, .. } => entries
                .iter()
                .map(|entry| Path::new(PROJECTS).join(entry))
                .collect(),
            Harness::Codex => CODEX_MEMORY.iter().map(PathBuf::from).collect(),
            Harness::Grok => GROK_MEMORY.iter().map(PathBuf::from).collect(),
            Harness::OpenCode => vec![PathBuf::from(OPENCODE_DATA)],
        }
    }
}

/// The `opencode.json` an OpenCode root is given, out of the account's own
/// `opencode.json` and `opencode.jsonc`, in that order, where there are any to
/// read.
///
/// The account's [`OPENCODE_CARRIED`] key and nothing else. A later file's
/// providers are merged over an earlier one's, the way opencode merges its
/// files. A file with comments or trailing commas is read the way opencode
/// reads it — see [`uncommented`]. An account with neither file, or none that
/// reads as a JSON object, is given a file with no provider in it.
fn opencode_config(account: [Option<String>; 2]) -> Vec<u8> {
    let mut written = Object::new();

    for text in account.iter().flatten() {
        let Ok(serde_json::Value::Object(own)) = serde_json::from_str(&uncommented(text)) else {
            continue;
        };

        if let Some(provider) = own.get(OPENCODE_CARRIED) {
            match written.get_mut(OPENCODE_CARRIED) {
                Some(kept) => merged_over(kept, provider),
                None => {
                    written.insert(OPENCODE_CARRIED.to_owned(), provider.clone());
                }
            }
        }
    }

    self::written(&serde_json::Value::Object(written))
}

/// `over` merged into `into`: objects key by key, all the way down, and
/// anything else replaced whole.
fn merged_over(into: &mut serde_json::Value, over: &serde_json::Value) {
    match (into, over) {
        (serde_json::Value::Object(into), serde_json::Value::Object(over)) => {
            for (key, value) in over {
                match into.get_mut(key) {
                    Some(kept) => merged_over(kept, value),
                    None => {
                        into.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (into, over) => *into = over.clone(),
    }
}

/// JSON with comments, as plain JSON: every `//` and `/* */` comment outside a
/// string taken out, and then every comma with only a closing bracket after it.
///
/// That is the whole of what JSONC adds to JSON, and how opencode reads its
/// `opencode.jsonc`. What is inside a string is kept as it is, escapes and all.
fn uncommented(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                plain.push(c);
                in_string(&mut chars, &mut plain);
            }
            '/' if chars.peek() == Some(&'/') => while chars.next_if(|c| *c != '\n').is_some() {},
            '/' if chars.peek() == Some(&'*') => {
                chars.next();

                let mut last = '\0';

                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }

                    last = c;
                }

                plain.push(' ');
            }
            _ => plain.push(c),
        }
    }

    // And the trailing commas, now that no comment can be between one and the
    // bracket after it.
    let mut kept = String::with_capacity(plain.len());
    let mut chars = plain.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                kept.push(c);
                in_string(&mut chars, &mut kept);
            }
            ',' => {
                let after = chars.clone().find(|c| !c.is_whitespace());

                if !matches!(after, Some('}' | ']')) {
                    kept.push(c);
                }
            }
            _ => kept.push(c),
        }
    }

    kept
}

/// The rest of a JSON string whose opening quote has been read, copied from
/// `chars` onto `onto` up to and including its closing quote.
fn in_string(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, onto: &mut String) {
    while let Some(c) = chars.next() {
        onto.push(c);

        match c {
            '\\' => onto.extend(chars.next()),
            '"' => return,
            _ => {}
        }
    }
}

/// The `config.toml` a Codex or a Grok Build root is given, out of the
/// account's own where there is one to read.
///
/// The `carried` keys of the account's own, as they are there, and nothing
/// else — [`CODEX_CARRIED`] or [`GROK_CARRIED`]. A key written `table.key` is
/// that one key of the table, in a table of the same name. An account with no
/// such file, or one that does not read as TOML, is given an empty file: a
/// session on the vendor's own provider needs nothing said.
fn toml_carrying(account: Option<&str>, carried: &[&str]) -> Vec<u8> {
    let mut written = toml::Table::new();

    if let Some(own) = account.and_then(|text| text.parse::<toml::Table>().ok()) {
        for key in carried {
            match key.split_once('.') {
                None => {
                    if let Some(value) = own.get(*key) {
                        written.insert((*key).to_owned(), value.clone());
                    }
                }
                Some((table, inner)) => {
                    let Some(value) = own
                        .get(table)
                        .and_then(toml::Value::as_table)
                        .and_then(|own| own.get(inner))
                    else {
                        continue;
                    };

                    if let toml::Value::Table(kept) = written
                        .entry(table)
                        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
                    {
                        kept.insert(inner.to_owned(), value.clone());
                    }
                }
            }
        }
    }

    toml::to_string(&written)
        .expect("a TOML table read off TOML writes as TOML")
        .into_bytes()
}

/// The settings a root is given, out of the account's own `settings.json` where
/// there is one to read.
///
/// **`skipDangerousModePermissionPrompt`**, which Claude Code 2.1.268 reads
/// from user settings. A session runs with its permissions bypassed and nobody
/// at its terminal, so a consent screen asking whether that is all right is a
/// session parked for ever — which is what a fresh account's first session was.
///
/// And the [`CARRIED`] keys of the account's own, as they are there. Nothing
/// else of it, and nothing at all of a file that is not a JSON object.
fn settings(account: Option<&[u8]>) -> Vec<u8> {
    let mut written = serde_json::Map::new();

    written.insert(
        "skipDangerousModePermissionPrompt".to_owned(),
        serde_json::Value::Bool(true),
    );

    if let Some(serde_json::Value::Object(own)) =
        account.and_then(|bytes| serde_json::from_slice(bytes).ok())
    {
        for key in CARRIED {
            if let Some(value) = own.get(key) {
                written.insert(key.to_owned(), value.clone());
            }
        }
    }

    self::written(&serde_json::Value::Object(written))
}

/// A JSON object, as `serde_json` holds one.
type Object = serde_json::Map<String, serde_json::Value>;

/// The `.claude.json` a session is given, out of the account's own where there
/// is one to read.
///
/// **Without `mcpServers`**, at the top level and under each `projects` entry.
///
/// **With a `projects` entry for each of `trusted` saying
/// `hasTrustDialogAccepted`**, beside whatever the account's entry for that path
/// already says. Claude Code 2.1.268 asks about the Repo's main checkout first
/// and then each directory up from the one it was started in, so a session in
/// the Worktree is past the trust dialog, with nobody at its terminal to answer
/// it. Not `bypassPermissionsModeAccepted`: 2.1.268 moves that key out of this
/// file, and the settings a root is given already answer that consent — see
/// [`settings`].
///
/// An account whose file is not there, or does not read as a JSON object, is
/// given the seeding and nothing else.
fn config(account: Option<&[u8]>, trusted: &[String]) -> Vec<u8> {
    let mut copy = match account.and_then(|bytes| serde_json::from_slice(bytes).ok()) {
        Some(serde_json::Value::Object(own)) => own,
        _ => Object::new(),
    };

    copy.remove(MCP_SERVERS);

    let projects = object_at(&mut copy, PROJECTS_CONFIG);

    for entry in projects.values_mut() {
        if let serde_json::Value::Object(entry) = entry {
            entry.remove(MCP_SERVERS);
        }
    }

    for path in trusted {
        object_at(projects, path).insert(TRUSTED.to_owned(), serde_json::Value::Bool(true));
    }

    written(&serde_json::Value::Object(copy))
}

/// What a session changed in its copy of `.claude.json`, merged into the
/// account's own file at `account`. `baseline` is the copy as the account last
/// had it, and `copy` is where the session left it.
///
/// **And a Grok Build session's copy of `auth.json` on Linux**, by the same rule
/// — see [`Root::login_copied`]. Its top-level keys are login scopes, so a
/// scope is what is merged, and neither of the keys below is ever one of them.
///
/// **`baseline` is moved on to the copy as it was merged**, once the account
/// has it. A copy can be shared by more than one launch — see
/// [`super::sharing`] — and the second of them to end is to carry only what
/// changed after the first, rather than write the first one's changes again
/// over whatever the account has had since.
///
/// **A merge rather than a copy.** A copy is never one file with the account's,
/// so writing it back whole would overwrite the account at every session end.
/// Sessions run side by side, and the human runs their own `claude` besides, so
/// the last session to end would win over whatever the others wrote in the
/// meantime — and a copy with no `mcpServers` in it would take the human's
/// servers away.
///
/// So only what the session changed goes back: each top-level key, and each
/// `projects` entry, whose value in the copy is not what it was in `baseline`.
/// A key the session removed is removed. `mcpServers` is neither written nor
/// removed, at either level, and an entry written back keeps the account's own.
/// The trust the copy was seeded with reaches the account only in an entry the
/// session changed.
///
/// **Nothing is written where nothing differs**, so a session that changed
/// nothing leaves the account's file byte for byte as it was. Where something
/// does, the file is written beside the account's and renamed over it, with the
/// account's mode. Says whether it was written.
///
/// An account with no such file is given one. A copy the session took away, or
/// a file on either side that does not read as a JSON object, is an error:
/// nothing is written over a file this cannot read.
///
/// Blocking: three reads at most, and a write and a rename where anything
/// changed.
pub(crate) fn merged_back(account: &Path, copy: &Path, baseline: &mut Vec<u8>) -> io::Result<bool> {
    let read = std::fs::read(copy)?;

    let was = object(baseline, "the copy as it was given")?;
    let now = object(&read, "the session's copy")?;

    let own = match std::fs::read(account) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };

    let mut merged = match &own {
        Some(bytes) => object(bytes, "the account's own")?,
        None => Object::new(),
    };

    if !merge(&was, &now, &mut merged) {
        *baseline = read;

        return Ok(false);
    }

    let mut beside = account.as_os_str().to_owned();
    beside.push(format!(".verkstead-{}", std::process::id()));
    let beside = PathBuf::from(beside);

    let replaced = std::fs::write(&beside, written(&serde_json::Value::Object(merged)))
        .and_then(|()| match own {
            Some(_) => std::fs::set_permissions(&beside, std::fs::metadata(account)?.permissions()),
            None => Ok(()),
        })
        .and_then(|()| std::fs::rename(&beside, account));

    if replaced.is_err() {
        let _ = std::fs::remove_file(&beside);
    } else {
        *baseline = read;
    }

    replaced.map(|()| true)
}

/// Into `account`, what differs between `baseline` and `now` — see
/// [`merged_back`] for the rule. Says whether anything did.
fn merge(baseline: &Object, now: &Object, account: &mut Object) -> bool {
    let mut changed = false;

    for key in keys(baseline, now) {
        if key == MCP_SERVERS || key == PROJECTS_CONFIG || baseline.get(key) == now.get(key) {
            continue;
        }

        changed = true;

        match now.get(key) {
            Some(value) => account.insert(key.to_owned(), value.clone()),
            None => account.remove(key),
        };
    }

    let empty = Object::new();
    let entries = |config: &'_ Object| match config.get(PROJECTS_CONFIG) {
        Some(serde_json::Value::Object(projects)) => projects.clone(),
        _ => empty.clone(),
    };
    let (was, is) = (entries(baseline), entries(now));

    for path in keys(&was, &is) {
        if was.get(path) == is.get(path) {
            continue;
        }

        changed = true;

        // An entry taken away that the account has no `projects` for is
        // nothing to take away, and no reason to give it an empty one.
        if !is.contains_key(path) && !account.get(PROJECTS_CONFIG).is_some_and(|p| p.is_object()) {
            continue;
        }

        let projects = object_at(account, PROJECTS_CONFIG);

        // The account's own servers for this path, which the copy never had.
        let servers = projects
            .get(path)
            .and_then(|entry| entry.get(MCP_SERVERS))
            .cloned();

        let mut entry = is.get(path).cloned();

        if let Some(serde_json::Value::Object(entry)) = &mut entry {
            entry.remove(MCP_SERVERS);
        }

        if let Some(servers) = servers {
            let kept = entry.get_or_insert_with(|| serde_json::Value::Object(Object::new()));

            if let serde_json::Value::Object(kept) = kept {
                kept.insert(MCP_SERVERS.to_owned(), servers);
            }
        }

        match entry {
            Some(entry) => projects.insert(path.to_owned(), entry),
            None => projects.remove(path),
        };
    }

    changed
}

/// The object under `key` in `object`, made an empty one first where there is
/// nothing there or something that is not an object.
fn object_at<'a>(object: &'a mut Object, key: &str) -> &'a mut Object {
    let value = object
        .entry(key)
        .or_insert_with(|| serde_json::Value::Object(Object::new()));

    if !value.is_object() {
        *value = serde_json::Value::Object(Object::new());
    }

    value
        .as_object_mut()
        .expect("made an object just above where it was not one")
}

/// Every key of either object, once each.
fn keys<'a>(one: &'a Object, other: &'a Object) -> std::collections::BTreeSet<&'a str> {
    one.keys().chain(other.keys()).map(String::as_str).collect()
}

/// `bytes` as a JSON object, or an error saying `what` does not read as one.
fn object(bytes: &[u8], what: &str) -> io::Result<Object> {
    match serde_json::from_slice(bytes) {
        Ok(serde_json::Value::Object(object)) => Ok(object),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{what} is JSON but not an object"),
        )),
        Err(error) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{what} does not read as JSON: {error}"),
        )),
    }
}

/// `value` as a file: indented by two spaces, as Claude writes one, with a line
/// ending after it.
fn written(value: &serde_json::Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).expect("a JSON value writes as JSON");
    bytes.push(b'\n');

    bytes
}

/// `path` resolved, or `path` as it stands where it cannot be.
fn resolved(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

/// The Repo's main checkout, out of its common git directory.
///
/// The directory holding it where it is called `.git`, and the directory itself
/// where it is not — a bare repository, which has no checkout to be beside.
/// That is Claude's own rule, read off 2.1.268: it follows the Worktree's
/// `.git` file to `commondir` and makes the same choice.
fn main_checkout(git_dir: &Path) -> PathBuf {
    match (git_dir.file_name(), git_dir.parent()) {
        (Some(name), Some(checkout)) if name == ".git" => checkout.to_owned(),
        _ => git_dir.to_owned(),
    }
}

/// The name Claude gives the `projects/` entry for `path`.
///
/// Read off Claude Code 2.1.268. Every UTF-16 unit outside `[a-zA-Z0-9]` becomes
/// `-`, one for one — so a character outside the Basic Multilingual Plane is two
/// of them. A name longer than 200 units is cut to its first 200, then `-`, then
/// a hash of the path in base 36: `h = (h << 5) - h + unit` kept to 32 bits and
/// made positive, over the path as it was rather than as it was renamed.
///
/// The plain path, never a `\\?\` spelling: that is a different string, and so
/// a different entry.
pub(crate) fn entry_named(path: &Path) -> String {
    let units: Vec<u16> = path.to_string_lossy().encode_utf16().collect();

    let mut named: String = units
        .iter()
        .map(|unit| match char::from_u32(u32::from(*unit)) {
            Some(kept) if kept.is_ascii_alphanumeric() => kept,
            _ => '-',
        })
        .collect();

    if named.len() <= LONGEST {
        return named;
    }

    // Every character in `named` is one ASCII byte by now, so the byte length
    // is the unit length Claude cuts at.
    named.truncate(LONGEST);
    named.push('-');
    named.push_str(&base36(u64::from(hashed(&units).unsigned_abs())));

    named
}

/// Java's string hash over `units`, in 32 bits.
fn hashed(units: &[u16]) -> i32 {
    units.iter().fold(0i32, |hash, unit| {
        (hash << 5)
            .wrapping_sub(hash)
            .wrapping_add(i32::from(*unit))
    })
}

/// `number` in base 36, the digits lower-case, as JavaScript writes it.
fn base36(mut number: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

    if number == 0 {
        return "0".to_owned();
    }

    let mut digits = Vec::new();

    while number > 0 {
        digits.push(DIGITS[(number % 36) as usize]);
        number /= 36;
    }

    digits.reverse();

    String::from_utf8(digits).expect("base 36 digits are ASCII")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Claude root's `projects/` entries.
    fn entries(root: &Root) -> Vec<String> {
        match &root.harness {
            Harness::Claude { entries, .. } => entries.clone(),
            Harness::Codex | Harness::Grok | Harness::OpenCode => {
                panic!("only a Claude root has `projects/` entries")
            }
        }
    }

    /// And the paths its `.claude.json` copy trusts.
    fn trusted(root: &Root) -> Vec<String> {
        match &root.harness {
            Harness::Claude { trusted, .. } => trusted.clone(),
            Harness::Codex | Harness::Grok | Harness::OpenCode => {
                panic!("only a Claude root has a `.claude.json` to trust in")
            }
        }
    }

    /// The name of the entry this very checkout's memory is under, which is
    /// the shape every entry has.
    #[test]
    fn an_entry_is_the_path_with_everything_but_letters_and_digits_as_dashes() {
        assert_eq!(
            entry_named(Path::new("/home/infi/src/verkstead")),
            "-home-infi-src-verkstead"
        );
        assert_eq!(
            entry_named(Path::new(
                "/var/lib/verkstead/worktrees/verkstead-built_roots.01"
            )),
            "-var-lib-verkstead-worktrees-verkstead-built-roots-01"
        );
    }

    /// One dash for each UTF-16 unit rather than for each character, which is
    /// what a JavaScript regular expression without the `u` flag replaces.
    #[test]
    fn a_character_outside_the_basic_plane_is_two_dashes() {
        assert_eq!(entry_named(Path::new("/tmp/é🦀")), "-tmp----");
    }

    /// A name longer than 200 is cut there, with the hash of the whole path on
    /// the end — the value here is what Claude Code 2.1.268 itself names it.
    #[test]
    fn a_path_longer_than_two_hundred_is_cut_and_hashed() {
        let path = format!("/{}", "a".repeat(250));

        let named = entry_named(Path::new(&path));

        assert_eq!(named.len(), 200 + 1 + "feo44x".len());
        assert_eq!(named, format!("-{}-feo44x", "a".repeat(199)));
    }

    /// The hash is made positive, including the one value whose negation does
    /// not fit in 32 bits.
    #[test]
    fn the_hash_is_javas_in_thirty_two_bits_and_written_positive() {
        let units: Vec<u16> = "hello".encode_utf16().collect();
        assert_eq!(hashed(&units), 99_162_322);

        let units: Vec<u16> = "polygenelubricants".encode_utf16().collect();
        assert_eq!(hashed(&units), i32::MIN);
        assert_eq!(base36(u64::from(i32::MIN.unsigned_abs())), "zik0zk");
    }

    #[test]
    fn a_main_checkout_is_the_directory_holding_its_git_directory() {
        assert_eq!(
            main_checkout(Path::new("/home/you/src/verkstead/.git")),
            Path::new("/home/you/src/verkstead")
        );
        assert_eq!(
            main_checkout(Path::new("/srv/git/verkstead.git")),
            Path::new("/srv/git/verkstead.git"),
            "and a bare repository is its own"
        );
    }

    fn read(bytes: &[u8]) -> serde_json::Value {
        serde_json::from_slice(bytes).unwrap()
    }

    /// An account with no settings of its own, or with some that do not read,
    /// is the account whose first session would otherwise park at the consent.
    #[test]
    fn the_bypass_key_is_written_whatever_the_account_has() {
        let bypass = serde_json::json!({ "skipDangerousModePermissionPrompt": true });

        assert_eq!(read(&settings(None)), bypass, "no settings.json at all");
        assert_eq!(
            read(&settings(Some(b"{ not json"))),
            bypass,
            "one that does not parse"
        );
        assert_eq!(
            read(&settings(Some(b"[1, 2]"))),
            bypass,
            "one that is not an object"
        );
        assert_eq!(
            read(&settings(Some(
                b"{\"skipDangerousModePermissionPrompt\": false}"
            ))),
            bypass,
            "and one that says otherwise is not asked"
        );
    }

    /// The two keys an API-key login needs come over as they are, and nothing
    /// else of the account's does.
    #[test]
    fn only_the_api_key_helper_and_the_environment_are_carried_over() {
        let account = serde_json::json!({
            "apiKeyHelper": "/usr/local/bin/print-key",
            "env": { "ANTHROPIC_BASE_URL": "https://proxy.example" },
            "hooks": { "Stop": [] },
            "enabledPlugins": { "the-humans@own": true },
            "permissions": { "allow": ["Bash"] },
            "statusLine": { "type": "command", "command": "true" },
        });

        assert_eq!(
            read(&settings(Some(account.to_string().as_bytes()))),
            serde_json::json!({
                "skipDangerousModePermissionPrompt": true,
                "apiKeyHelper": "/usr/local/bin/print-key",
                "env": { "ANTHROPIC_BASE_URL": "https://proxy.example" },
            })
        );
    }

    /// The copy keeps the account's own entry for a trusted path and adds the
    /// trust to it; a Windows path is keyed with forward slashes, as Claude
    /// keys one.
    #[test]
    fn the_copy_trusts_each_path_beside_what_its_entry_already_says() {
        let root = Root::claude(
            Platform::Windows,
            Path::new(r"C:\Users\ada\.claude"),
            Path::new(r"\\?\C:\Users\ada\src\verkstead"),
            Path::new(r"\\?\C:\ProgramData\Verkstead\worktrees\verkstead-x"),
        );

        assert_eq!(
            trusted(&root),
            [
                "C:/Users/ada/src/verkstead",
                "C:/ProgramData/Verkstead/worktrees/verkstead-x"
            ]
        );

        let account = serde_json::json!({
            "mcpServers": { "the-humans": {} },
            "projects": {
                "C:/Users/ada/src/verkstead": {
                    "allowedTools": ["Bash"],
                    "mcpServers": { "its-own": {} },
                },
            },
        });

        assert_eq!(
            read(&config(
                Some(account.to_string().as_bytes()),
                &trusted(&root)
            )),
            serde_json::json!({
                "projects": {
                    "C:/Users/ada/src/verkstead": {
                        "allowedTools": ["Bash"],
                        "hasTrustDialogAccepted": true,
                    },
                    "C:/ProgramData/Verkstead/worktrees/verkstead-x": {
                        "hasTrustDialogAccepted": true,
                    },
                },
            })
        );
        assert_eq!(
            read(&config(Some(b"{ not json"), &trusted(&root)[..1])),
            serde_json::json!({
                "projects": { "C:/Users/ada/src/verkstead": { "hasTrustDialogAccepted": true } },
            }),
            "and an account file that does not read gives the seeding alone"
        );
    }

    /// Merge `now` over `account`, with `baseline` as the copy was given.
    fn merging(
        baseline: serde_json::Value,
        now: serde_json::Value,
        account: serde_json::Value,
    ) -> (bool, serde_json::Value) {
        let mut account = account.as_object().unwrap().clone();
        let changed = merge(
            baseline.as_object().unwrap(),
            now.as_object().unwrap(),
            &mut account,
        );

        (changed, serde_json::Value::Object(account))
    }

    /// An entry the session changed goes back whole but for its MCP servers,
    /// which stay the account's; an entry it took away leaves those behind; an
    /// entry it did not touch is the account's as it is now.
    #[test]
    fn an_entry_merged_back_keeps_the_accounts_own_mcp_servers() {
        let (changed, merged) = merging(
            serde_json::json!({ "projects": {
                "/changed": { "hasTrustDialogAccepted": true },
                "/removed": { "allowedTools": [] },
                "/untouched": { "allowedTools": [] },
            }}),
            serde_json::json!({ "projects": {
                "/changed": { "hasTrustDialogAccepted": true, "lastCost": 1, "mcpServers": { "added-inside": {} } },
                "/untouched": { "allowedTools": [] },
            }}),
            serde_json::json!({
                "mcpServers": { "the-humans": {} },
                "projects": {
                    "/changed": { "mcpServers": { "its-own": {} } },
                    "/removed": { "allowedTools": [], "mcpServers": { "kept": {} } },
                    "/untouched": { "allowedTools": ["changed by the account"] },
                },
            }),
        );

        assert!(changed);
        assert_eq!(
            merged,
            serde_json::json!({
                "mcpServers": { "the-humans": {} },
                "projects": {
                    "/changed": {
                        "hasTrustDialogAccepted": true,
                        "lastCost": 1,
                        "mcpServers": { "its-own": {} },
                    },
                    "/removed": { "mcpServers": { "kept": {} } },
                    "/untouched": { "allowedTools": ["changed by the account"] },
                },
            })
        );
    }

    /// A copy the session left as it was given changes nothing, and neither
    /// does an `mcpServers` a session wrote into it.
    #[test]
    fn nothing_changed_but_mcp_servers_is_nothing_to_merge() {
        let given = serde_json::json!({ "numStartups": 1, "projects": { "/repo": {} } });

        let (changed, _) = merging(given.clone(), given.clone(), serde_json::json!({}));
        assert!(!changed);

        let (changed, merged) = merging(
            given,
            serde_json::json!({
                "numStartups": 1,
                "mcpServers": { "added-inside": {} },
                "projects": { "/repo": {} },
            }),
            serde_json::json!({ "mcpServers": { "the-humans": {} } }),
        );
        assert!(!changed);
        assert_eq!(
            merged,
            serde_json::json!({ "mcpServers": { "the-humans": {} } })
        );
    }

    /// The write-back itself: nothing written where nothing changed, a file
    /// given to an account with none, the mode kept, and nothing written over a
    /// file that does not read.
    #[cfg(unix)]
    #[test]
    fn a_merge_is_written_by_rename_with_the_accounts_mode_and_never_over_what_does_not_read() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let (account, copy) = (
            dir.path().join(".claude.json"),
            dir.path().join("copy.json"),
        );
        let mut baseline = b"{\"numStartups\": 1}\n".to_vec();

        std::fs::write(&copy, &baseline).unwrap();
        assert!(!merged_back(&account, &copy, &mut baseline).unwrap());
        assert!(!account.exists(), "nothing changed, so nothing is written");

        std::fs::write(&copy, "{\"numStartups\": 2}\n").unwrap();
        assert!(merged_back(&account, &copy, &mut baseline).unwrap());
        assert_eq!(
            read(&std::fs::read(&account).unwrap()),
            serde_json::json!({ "numStartups": 2 })
        );

        std::fs::set_permissions(&account, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(&copy, "{\"numStartups\": 3}\n").unwrap();
        assert!(merged_back(&account, &copy, &mut baseline).unwrap());
        assert_eq!(
            std::fs::metadata(&account).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            2,
            "and nothing is left beside it"
        );

        std::fs::write(&account, "{ half written").unwrap();
        assert!(merged_back(&account, &copy, &mut baseline).is_err());
        assert_eq!(std::fs::read_to_string(&account).unwrap(), "{ half written");
    }

    /// A copy two launches share is merged as each ends, and the second merge
    /// carries only what changed after the first — not the first one's changes
    /// again, over what the account has had since.
    #[test]
    fn a_second_merge_of_one_copy_carries_only_what_changed_after_the_first() {
        let dir = tempfile::tempdir().unwrap();
        let (account, copy) = (
            dir.path().join(".claude.json"),
            dir.path().join("copy.json"),
        );
        let mut baseline = b"{\"numStartups\": 1, \"theme\": \"dark\"}\n".to_vec();

        std::fs::write(&account, &baseline).unwrap();
        std::fs::write(&copy, "{\"numStartups\": 2, \"theme\": \"dark\"}\n").unwrap();
        assert!(merged_back(&account, &copy, &mut baseline).unwrap());

        // The human's own `claude` starts once more in between.
        std::fs::write(&account, "{\"numStartups\": 5, \"theme\": \"dark\"}\n").unwrap();

        assert!(
            !merged_back(&account, &copy, &mut baseline).unwrap(),
            "nothing changed in the copy since the first merge"
        );

        std::fs::write(&copy, "{\"numStartups\": 2, \"theme\": \"light\"}\n").unwrap();
        assert!(merged_back(&account, &copy, &mut baseline).unwrap());
        assert_eq!(
            read(&std::fs::read(&account).unwrap()),
            serde_json::json!({ "numStartups": 5, "theme": "light" }),
            "and what did change is all that goes back"
        );
    }

    /// A Worktree and a Repo whose entries are one name join it once.
    #[test]
    fn the_repos_entry_and_the_worktrees_are_joined_once_each() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(&worktree).unwrap();

        let account = dir.path().join("account/.claude");
        let root = Root::claude(Platform::Linux, &account, &repo.join(".git"), &worktree);

        root.made_in_account().unwrap();

        let joined = root.joined(Path::new("/inside"));
        let repo_entry = entry_named(&repo);
        let worktree_entry = entry_named(&worktree);

        assert_eq!(
            joined,
            [
                (
                    account.join("projects").join(&repo_entry),
                    PathBuf::from("/inside/.claude/projects").join(&repo_entry),
                ),
                (
                    account.join("projects").join(&worktree_entry),
                    PathBuf::from("/inside/.claude/projects").join(&worktree_entry),
                ),
            ],
            "no credentials file in the account, so none in the root"
        );
        assert!(account.join("projects").join(&repo_entry).is_dir());
        assert!(account.join("projects").join(&worktree_entry).is_dir());

        std::fs::write(account.join(CREDENTIALS), "{}\n").unwrap();
        assert_eq!(
            root.joined(Path::new("/inside"))[0],
            (
                account.join(CREDENTIALS),
                PathBuf::from("/inside/.claude/.credentials.json")
            )
        );

        let same = Root::claude(Platform::Linux, &account, &worktree.join(".git"), &worktree);
        assert_eq!(entries(&same), [worktree_entry]);
    }

    /// With memory off, only the credentials are joined, and nothing is made
    /// under the account's `projects/`.
    #[test]
    fn a_root_without_memory_joins_no_entry_and_makes_none() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let worktree = dir.path().join("worktree");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(&worktree).unwrap();

        let account = dir.path().join("account/.claude");
        std::fs::create_dir_all(&account).unwrap();
        std::fs::write(account.join(CREDENTIALS), "{}\n").unwrap();

        let root = Root::claude(Platform::Linux, &account, &repo.join(".git"), &worktree)
            .remembering(false);
        assert_eq!(
            root.unshared_in(Path::new("/built")),
            [PathBuf::from("/built/.claude/projects")]
        );

        root.made_in_account().unwrap();
        assert!(
            !account.join("projects").exists(),
            "nothing is made in the account for a root that joins none of it"
        );

        assert_eq!(
            root.joined(Path::new("/inside")),
            [(
                account.join(CREDENTIALS),
                PathBuf::from("/inside/.claude/.credentials.json")
            )]
        );
    }

    /// A path resolved on Windows carries `\\?\` in front of it, and its entry is
    /// named from the plain path a session is started in.
    #[test]
    fn a_verbatim_path_is_named_as_the_plain_path_it_spells() {
        let root = Root::claude(
            Platform::Windows,
            Path::new(r"C:\Users\ada\.claude"),
            Path::new(r"\\?\C:\Users\ada\src\verkstead"),
            Path::new(r"\\?\C:\ProgramData\Verkstead\worktrees\verkstead-x"),
        );

        assert_eq!(
            entries(&root),
            [
                "C--Users-ada-src-verkstead",
                "C--ProgramData-Verkstead-worktrees-verkstead-x"
            ]
        );
    }

    /// A Worktree reached through a link is named as a session will find itself
    /// standing in it: through the link on Linux, where the bind makes the path
    /// as it was stored, and resolved on a Mac, where the path is the host's.
    #[cfg(unix)]
    #[test]
    fn a_worktree_is_named_as_the_session_inside_will_read_its_path() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real/worktree");
        let linked = dir.path().join("linked");
        std::fs::create_dir_all(&real).unwrap();
        std::os::unix::fs::symlink(dir.path().join("real"), &linked).unwrap();

        let account = dir.path().join("account/.claude");
        let git_dir = dir.path().join("repo/.git");
        let through = linked.join("worktree");

        assert_eq!(
            entries(&Root::claude(Platform::Linux, &account, &git_dir, &through))[1],
            entry_named(&through)
        );
        assert_eq!(
            entries(&Root::claude(Platform::MacOs, &account, &git_dir, &through))[1],
            entry_named(&real.canonicalize().unwrap())
        );
    }

    /// A Codex root joins the login and the two memory directories, made in
    /// the account first; with memory off, the login alone, nothing made in the
    /// account, and the two directories the root's own.
    #[test]
    fn a_codex_root_joins_its_login_and_its_memory_by_the_switch() {
        let dir = tempfile::tempdir().unwrap();
        let account = dir.path().join("account/.codex");
        std::fs::create_dir_all(&account).unwrap();

        let forgetting = Root::codex(&account).remembering(false);
        forgetting.made_in_account().unwrap();

        assert!(
            forgetting.joined(Path::new("/inside")).is_empty(),
            "no login in the account and no memory shared, so nothing is joined"
        );
        assert!(!account.join("sessions").exists() && !account.join("memories").exists());
        assert_eq!(
            forgetting.unshared_in(Path::new("/built")),
            [
                PathBuf::from("/built/.codex/sessions"),
                PathBuf::from("/built/.codex/memories")
            ]
        );

        std::fs::write(account.join(AUTH), "{}\n").unwrap();

        let remembering = Root::codex(&account);
        remembering.made_in_account().unwrap();

        assert_eq!(
            remembering.joined(Path::new("/inside")),
            [
                (
                    account.join(AUTH),
                    PathBuf::from("/inside/.codex/auth.json")
                ),
                (
                    account.join("sessions"),
                    PathBuf::from("/inside/.codex/sessions")
                ),
                (
                    account.join("memories"),
                    PathBuf::from("/inside/.codex/memories")
                ),
            ]
        );
        assert!(account.join("sessions").is_dir() && account.join("memories").is_dir());
        assert!(remembering.unshared_in(Path::new("/built")).is_empty());
    }

    fn table(bytes: &[u8]) -> toml::Table {
        std::str::from_utf8(bytes).unwrap().parse().unwrap()
    }

    /// The two keys a custom provider needs come over as they are, and nothing
    /// else of the account's does.
    #[test]
    fn only_the_model_provider_keys_are_carried_into_codexs_config() {
        let account = r#"
            model = "gpt-5-codex"
            model_provider = "proxy"
            notify = ["notify-send"]

            [model_providers.proxy]
            name = "The proxy"
            base_url = "https://proxy.example/v1"
            env_key = "PROXY_API_KEY"

            [mcp_servers.the-humans]
            command = "npx"

            [profiles.fast]
            model = "gpt-5-mini"

            [projects."/home/you/src/verkstead"]
            trust_level = "trusted"

            [features]
            memories = true
        "#;

        assert_eq!(
            table(&toml_carrying(Some(account), &CODEX_CARRIED)),
            toml::toml! {
                model_provider = "proxy"

                [model_providers.proxy]
                name = "The proxy"
                base_url = "https://proxy.example/v1"
                env_key = "PROXY_API_KEY"
            }
        );
    }

    /// An account with no `config.toml`, or one that does not read, is given an
    /// empty one.
    #[test]
    fn an_account_with_no_codex_config_to_read_is_given_an_empty_one() {
        assert!(toml_carrying(None, &CODEX_CARRIED).is_empty());
        assert!(toml_carrying(Some("model_provider = "), &CODEX_CARRIED).is_empty());
        assert!(toml_carrying(Some("model = \"gpt-5-codex\"\n"), &CODEX_CARRIED).is_empty());
    }

    /// A Grok Build root joins the login and its two memory directories, made
    /// in the account first; with memory off, the login alone, nothing made in
    /// the account, and the two directories the root's own.
    #[test]
    fn a_grok_root_joins_its_login_and_its_memory_by_the_switch() {
        let dir = tempfile::tempdir().unwrap();
        let account = dir.path().join("account/.grok");
        std::fs::create_dir_all(&account).unwrap();
        std::fs::write(account.join(AUTH), "{}\n").unwrap();

        let forgetting = Root::grok(&account).remembering(false);
        forgetting.made_in_account().unwrap();

        assert_eq!(
            forgetting.joined(Path::new("/inside")),
            [(account.join(AUTH), PathBuf::from("/inside/.grok/auth.json"))]
        );
        assert!(!account.join("sessions").exists() && !account.join("memory").exists());
        assert_eq!(
            forgetting.unshared_in(Path::new("/built")),
            [
                PathBuf::from("/built/.grok/sessions"),
                PathBuf::from("/built/.grok/memory")
            ]
        );

        let remembering = Root::grok(&account);
        remembering.made_in_account().unwrap();

        assert_eq!(
            remembering.joined(Path::new("/inside")),
            [
                (account.join(AUTH), PathBuf::from("/inside/.grok/auth.json")),
                (
                    account.join("sessions"),
                    PathBuf::from("/inside/.grok/sessions")
                ),
                (
                    account.join("memory"),
                    PathBuf::from("/inside/.grok/memory")
                ),
            ]
        );
        assert!(account.join("sessions").is_dir() && account.join("memory").is_dir());
        assert!(remembering.unshared_in(Path::new("/built")).is_empty());
    }

    /// Only a Grok Build root on Linux is given its login as a copy: grok saves
    /// one by a rename a bind refuses, and the other harnesses and platforms are
    /// served by a link.
    #[test]
    fn only_a_grok_login_on_linux_is_copied() {
        let account = Path::new("/home/you/.grok");
        let grok = Root::grok(account);

        assert!(grok.login_copied(Platform::Linux));
        assert!(!grok.login_copied(Platform::MacOs));
        assert!(!grok.login_copied(Platform::Windows));
        assert!(!Root::codex(Path::new("/home/you/.codex")).login_copied(Platform::Linux));
        assert!(
            !Root::claude(
                Platform::Linux,
                Path::new("/home/you/.claude"),
                Path::new("/home/you/src/repo/.git"),
                Path::new("/home/you/src/repo"),
            )
            .login_copied(Platform::Linux)
        );
    }

    /// What reaches a model and signs in to one comes over as it is, the models
    /// endpoints alone of their table, and nothing else of the account's.
    #[test]
    fn only_what_reaches_a_model_is_carried_into_groks_config() {
        let account = r#"
            [models]
            default = "the-proxy"

            [model.the-proxy]
            model = "gpt-5"
            base_url = "https://proxy.example/v1"
            env_key = "PROXY_API_KEY"
            auth_provider = "vault"

            [model_providers.proxy]
            base_url = "https://proxy.example/v1"

            [auth_provider.vault]
            command = "/usr/local/bin/print-token"

            [auth]
            auth_provider_command = "/usr/local/bin/sign-in"

            [grok_com_config.oidc]
            issuer = "https://id.example"

            [endpoints]
            models_base_url = "https://proxy.example/v1"
            models_list_url = "https://proxy.example/v1/models"
            trace_upload_bucket = "s3://the-humans"

            [mcp_servers.the-humans]
            command = "npx"

            [memory]
            enabled = true

            [ui]
            screen_mode = "minimal"

            [[hooks.Stop]]
            command = "notify-send"
        "#;

        assert_eq!(
            table(&toml_carrying(Some(account), &GROK_CARRIED)),
            toml::toml! {
                [model.the-proxy]
                model = "gpt-5"
                base_url = "https://proxy.example/v1"
                env_key = "PROXY_API_KEY"
                auth_provider = "vault"

                [model_providers.proxy]
                base_url = "https://proxy.example/v1"

                [auth_provider.vault]
                command = "/usr/local/bin/print-token"

                [auth]
                auth_provider_command = "/usr/local/bin/sign-in"

                [grok_com_config.oidc]
                issuer = "https://id.example"

                [endpoints]
                models_base_url = "https://proxy.example/v1"
                models_list_url = "https://proxy.example/v1/models"
            }
        );
    }

    /// An account with no `config.toml`, one that does not read, or one with
    /// nothing a model is reached by, is given an empty one — and an
    /// `endpoints` table with none of the two keys is not written at all.
    #[test]
    fn an_account_with_nothing_to_carry_into_groks_config_is_given_an_empty_one() {
        assert!(toml_carrying(None, &GROK_CARRIED).is_empty());
        assert!(toml_carrying(Some("[model"), &GROK_CARRIED).is_empty());
        assert!(
            toml_carrying(
                Some("[endpoints]\ntrace_upload_bucket = \"s3://x\"\n"),
                &GROK_CARRIED
            )
            .is_empty()
        );
        assert!(toml_carrying(Some("endpoints = \"not a table\"\n"), &GROK_CARRIED).is_empty());
    }

    /// An OpenCode root shares its data directory whole and builds its config
    /// directory; with memory off it builds both, and links the login alone
    /// into the data directory. Nothing is made in the account for a root that
    /// joins none of its store.
    #[test]
    fn an_opencode_root_joins_its_data_directory_whole_or_its_login_alone() {
        let dir = tempfile::tempdir().unwrap();
        let account = dir.path().join("account");
        std::fs::create_dir_all(account.join(".local/share")).unwrap();

        let forgetting = Root::opencode(&account).remembering(false);
        forgetting.made_in_account().unwrap();

        assert_eq!(
            forgetting.built(),
            [".config/opencode", ".local/share/opencode"]
        );
        assert!(
            forgetting.joined(Path::new("/inside")).is_empty(),
            "no login in the account and no memory shared, so nothing is joined"
        );
        assert!(!account.join(".local/share/opencode").exists());
        assert!(forgetting.unshared_in(Path::new("/built")).is_empty());

        std::fs::create_dir_all(account.join(".local/share/opencode")).unwrap();
        std::fs::write(account.join(OPENCODE_AUTH), "{}\n").unwrap();

        assert!(forgetting.login_alone());
        assert_eq!(
            forgetting.joined(Path::new("/inside")),
            [(
                account.join(OPENCODE_AUTH),
                PathBuf::from("/inside/.local/share/opencode/auth.json")
            )]
        );
        assert_eq!(
            forgetting.credentials_in(Path::new("/built")),
            Path::new("/built/.local/share/opencode/auth.json")
        );

        let remembering = Root::opencode(&account);

        assert_eq!(remembering.built(), [".config/opencode"]);
        assert!(!remembering.login_alone());
        assert_eq!(
            remembering.joined(Path::new("/inside")),
            [(
                account.join(".local/share/opencode"),
                PathBuf::from("/inside/.local/share/opencode")
            )],
            "the data directory whole, with the login inside it"
        );
        assert_eq!(
            remembering.written(Path::new("/built")).0,
            Path::new("/built/.config/opencode/opencode.json")
        );

        assert_eq!(
            Root::login_of(
                &crate::store::Account::OpenCode {
                    home: account.clone()
                },
                true
            ),
            None
        );
        assert_eq!(
            Root::login_of(
                &crate::store::Account::OpenCode {
                    home: account.clone()
                },
                false
            ),
            Some(account.join(OPENCODE_AUTH))
        );
    }

    /// The provider comes over as it is, and nothing else of the account's.
    #[test]
    fn only_the_provider_is_carried_into_opencodes_config() {
        let account = serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "provider": {
                "proxy": {
                    "npm": "@ai-sdk/openai-compatible",
                    "options": { "baseURL": "https://proxy.example/v1" },
                    "models": { "gpt-5": {} },
                },
            },
            "mcp": { "the-humans": { "type": "local", "command": ["npx"] } },
            "plugin": ["the-humans-plugin"],
            "agent": { "review": {} },
            "command": { "ship": {} },
            "instructions": ["RULES.md"],
            "theme": "tokyonight",
        });

        assert_eq!(
            read(&opencode_config([Some(account.to_string()), None])),
            serde_json::json!({
                "provider": {
                    "proxy": {
                        "npm": "@ai-sdk/openai-compatible",
                        "options": { "baseURL": "https://proxy.example/v1" },
                        "models": { "gpt-5": {} },
                    },
                },
            })
        );
    }

    /// An `opencode.jsonc` with comments and trailing commas still gives its
    /// provider, merged over the `opencode.json` beside it.
    #[test]
    fn an_opencode_jsonc_with_comments_still_gives_its_provider() {
        let json = r#"{ "provider": { "proxy": { "options": { "baseURL": "https://old.example", "timeout": 5 } } } }"#;
        let jsonc = r#"
            // The human's own proxy.
            {
              "provider": {
                /* where it is */
                "proxy": { "options": { "baseURL": "https://proxy.example/v1", }, },
                "other": { "name": "a // not a comment, \" nor /* this */" },
              },
              "mcp": {},
            }
        "#;

        assert_eq!(
            read(&opencode_config([
                Some(json.to_owned()),
                Some(jsonc.to_owned())
            ])),
            serde_json::json!({
                "provider": {
                    "proxy": { "options": { "baseURL": "https://proxy.example/v1", "timeout": 5 } },
                    "other": { "name": "a // not a comment, \" nor /* this */" },
                },
            })
        );
    }

    /// Each harness's own global instructions file, inside a directory its root
    /// already builds, holding the settings text and nothing else — and no file
    /// anywhere for a text nobody typed.
    #[test]
    fn every_root_is_given_the_settings_text_as_the_file_its_harness_reads() {
        let account = Path::new("/home/you/.claude");
        let text = "Prefer the smallest change.\n\n- And say why.\n";

        let claude = Root::claude(
            Platform::Linux,
            account,
            Path::new("/home/you/src/verkstead/.git"),
            Path::new("/state/worktrees/verkstead-x"),
        );

        let built = Path::new("/built");

        for (root, file) in [
            (claude, "/built/.claude/CLAUDE.md"),
            (Root::codex(account), "/built/.codex/AGENTS.md"),
            (Root::grok(account), "/built/.grok/AGENTS.md"),
            (Root::opencode(account), "/built/.config/opencode/AGENTS.md"),
        ] {
            assert_eq!(
                root.instructed(built, text),
                Some((PathBuf::from(file), text.as_bytes().to_vec())),
                "the text verbatim, with no heading over it and nothing saying \
                 where it came from"
            );
            assert_eq!(
                root.remembering(false).instructed(built, ""),
                None,
                "and nothing at all in a root for a setting nobody typed"
            );
        }
    }

    /// An account with no config, or one that does not read, is given a file
    /// with no provider in it.
    #[test]
    fn an_account_with_no_opencode_config_to_read_is_given_one_with_no_provider() {
        let empty = serde_json::json!({});

        assert_eq!(read(&opencode_config([None, None])), empty);
        assert_eq!(
            read(&opencode_config([Some("{ not json".to_owned()), None])),
            empty
        );
        assert_eq!(
            read(&opencode_config([None, Some("[1, 2]".to_owned())])),
            empty
        );
    }
}
