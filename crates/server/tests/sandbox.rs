//! What a session can reach, asked by running a probe inside a Conversation's
//! sandbox.
//!
//! Nothing here reads the flags the sandbox was built with. The flags *are* what
//! is being tested, and a test that asserts them asserts itself — it would go on
//! passing while bwrap changed what one of them meant, or while a later bind
//! quietly mounted something over another. What settles whether the rest of the
//! machine is still reachable is a command inside the sandbox trying to reach
//! it, and reporting what happened.
//!
//! The probe is a shell script that prints one `key=value` line per fact. Each
//! path comes back as `write`, `read`, or `absent`, and the difference between
//! the three is attempted rather than asked of the metadata: a read-only bind
//! and a directory somebody has no write permission on look identical to
//! `test -w`, and only one of them is the surface being described.
//!
//! **This is the Linux half.** What the same description comes to on a Mac is
//! `tests/sandbox_macos.rs`, which asks the same questions of Apple's own
//! mechanism and reads a different answer back from the paths a session may not
//! reach. Neither suite runs on the other's machine: what is asserted here is a
//! mount namespace, and there is none of one to probe anywhere else.

#![cfg(target_os = "linux")]

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::handoffs::Handoffs;
use verkstead_server::key::WorkbenchKey;
use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{
    Bind, Closing, Executable, Homes, Reachable, Sandbox, SandboxConfig, under_dev_shell,
};
use verkstead_server::settings::{RustBuildCache, Settings};
use verkstead_server::skills::Skills;
use verkstead_server::store;

/// Where the server this Conversation belongs to is listening — which is what a
/// session inside is told to put its Question Sets to.
const LISTENING: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8422);

/// What the fixture's Codex account keeps in its `config.toml`: a provider of
/// its own, which a session needs to reach the model, and the human's own MCP
/// servers and features, which a session is not given.
const CODEX_ACCOUNT_CONFIG: &str = r#"model_provider = "proxy"
notify = ["notify-send"]

[model_providers.proxy]
name = "The proxy"
base_url = "https://proxy.example/v1"

[mcp_servers.the-humans]
command = "npx"

[features]
memories = true
"#;

/// What the fixture's Grok Build account keeps in its `config.toml`: a model
/// behind a proxy of its own and how it signs in, which a session needs, and
/// the human's own MCP servers, memory setting and interface, which a session
/// is not given.
const GROK_ACCOUNT_CONFIG: &str = r#"[model.the-proxy]
model = "gpt-5"
base_url = "https://proxy.example/v1"

[auth]
auth_provider_command = "/usr/local/bin/sign-in"

[mcp_servers.the-humans]
command = "npx"

[memory]
enabled = true

[ui]
screen_mode = "minimal"
"#;

/// And the login it keeps: one scope, as grok 1.0.13 writes `auth.json`, whose
/// top-level keys are the scopes a login is for.
const GROK_LOGIN: &str = "{\"https://auth.x.ai::the-humans\": {\"key\": \"the login\"}}\n";

/// What the fixture's OpenCode account keeps in its `opencode.jsonc`, comments
/// and all: a provider of its own, which a session needs, and the human's own
/// MCP servers, plugins and instructions, which a session is not given.
const OPENCODE_ACCOUNT_CONFIG: &str = r#"{
  // The human's own proxy.
  "provider": {
    "proxy": {
      "npm": "@ai-sdk/openai-compatible",
      "options": { "baseURL": "https://proxy.example/v1" },
    },
  },
  /* How the human works. */
  "mcp": { "the-humans": { "type": "local", "command": ["npx"] } },
  "plugin": ["the-humans-plugin"],
  "instructions": ["RULES.md"],
}
"#;

/// And the login it keeps.
const OPENCODE_LOGIN: &str = "{\"opencode\": {\"type\": \"api\", \"key\": \"the login\"}}\n";

/// What stands in for the server's own image: an executable that says which
/// build it is.
///
/// A real server equips a session with the binary it is itself running, so that
/// the CLI a session asks with and the server it asks cannot disagree about a
/// schema. A test harness's own image is the test harness, which would prove
/// nothing about *which* binary arrived — so the fixture writes one of its own
/// and hands the sandbox that. The bind is the same either way, and this one
/// answers in words a probe can recognise.
const SAYS_WHICH_BUILD: &str = "#!/bin/sh\nprintf 'verkstead 0.0.0-the-servers-own\\n'\n";

/// And what stands in for the sccache the server resolved, for the same reason:
/// what has to be shown is that the binary a session compiles through is the
/// one the server found, which a real sccache could not say.
///
/// That is the half a *session* runs, as a client. The stub answers as the
/// other half too — see [`Grilling::sccache`] — because an sccache is a client
/// and a server, and which sandbox the server is in is the whole of what
/// [`the_compile_server_holds_the_worktrees_and_none_of_the_data_directory`]
/// has to settle.
const SAYS_WHICH_SCCACHE: &str = "printf 'sccache 0.0.0-the-one-resolved\\n'\n";

/// What the compile server's report is called inside the build cache, which is
/// the one directory both it and this test can write to and read.
const COMPILE_SERVER_REPORT: &str = "compile-server-report";

/// A Conversation part-way through its first grilling: a Repo in a directory of
/// its own, a Profile to run as, and a worktree under Verkstead's own state
/// directory.
///
/// Everything is real. The repository is a repository, the worktree is one git
/// made, and the Conversation is a row the store wrote — because what the
/// sandbox binds is read off those, and a fixture that hand-built the paths
/// would prove the probe works rather than that the sandbox does.
struct Grilling {
    /// Kept alive for as long as the fixture is: the directories go when these
    /// drop, and a worktree that vanished mid-probe would fail obscurely.
    elsewhere: tempfile::TempDir,
    state: tempfile::TempDir,
    home: tempfile::TempDir,

    /// Where the Repo is, and the sibling checkout beside it that no session has
    /// any business seeing.
    repo: PathBuf,
    sibling: PathBuf,

    conversation: store::Conversation,
    profile: store::Profile,

    /// The store the Conversation was written into, for the test that stands a
    /// real server up over it and lets a session inside the sandbox ask it
    /// something.
    pool: sqlx::SqlitePool,

    /// The bundled skills, installed where the server installs them: under the
    /// Data Directory, at startup.
    skills: Skills,

    /// And the executable a session asks with, which for a real server is its
    /// own image — see [`SAYS_WHICH_BUILD`] for what stands in for one here.
    verkstead: Executable,

    /// And where the handoff documents go, which is a root under the same
    /// directory — one directory per Conversation, made as its sandbox is built.
    handoffs: Handoffs,

    /// And where the files the human attached to it are, which is a root under
    /// that directory again — one directory per Conversation, and read-only
    /// inside every session the Conversation has.
    attachments: Attachments,

    /// The settings files, in that directory again. Nothing is in them until a
    /// test says so — see [`Grilling::configure_github_token`] and
    /// [`Grilling::configure`] — which is what an installation nobody
    /// has been to the settings page of looks like.
    settings: Settings,
}

impl Grilling {
    /// The sandbox this Conversation's session would run in, with `extra` as
    /// whatever Sandbox Configuration asked for.
    ///
    /// With no shared build cache, which is what every test here that is not
    /// about one wants: the cache is a bind and four variables, and a test
    /// asking what else is inside should not have to know about them.
    fn sandbox(&self, extra: Vec<Bind>) -> Sandbox {
        self.sandbox_reaching(LISTENING, &BuildCache::none(), extra)
    }

    /// And one built around `cache`, which is what the tests about the build
    /// cache ask for. What is switched on and how big it may grow is read out
    /// of `config.yaml` as the sandbox is built, so a test says that by writing
    /// the file — see [`Grilling::configure`].
    fn sandbox_caching(&self, cache: &BuildCache) -> Sandbox {
        self.sandbox_reaching(LISTENING, cache, vec![])
    }

    /// Where the shared build cache is on the host, which is a directory of the
    /// fixture's own rather than the machine's XDG one.
    fn cache_dir(&self) -> PathBuf {
        self.state.path().join("build-cache")
    }

    /// A cache at that directory, with a stub sccache where `compiling` says so.
    ///
    /// The stub is a script that says which build it is, for the reason
    /// [`SAYS_WHICH_BUILD`] is one: what has to be shown is that the file the
    /// server resolved is the file a session finds at
    /// `/verkstead/bin/sccache`, and a real sccache would answer that question
    /// with whatever the machine happened to have installed.
    fn cache(&self, compiling: bool) -> BuildCache {
        let dir = self.cache_dir();
        std::fs::create_dir_all(&dir).unwrap();

        BuildCache::at(
            dir,
            compiling.then(|| self.sccache()),
            self.state.path().to_owned(),
        )
    }

    /// The Worktrees directory the compile server is shown of that Data
    /// Directory, which is where this fixture's own worktree already is.
    fn worktrees_dir(&self) -> PathBuf {
        self.state.path().join("worktrees")
    }

    /// The stub sccache, written where the server would have found a real one.
    ///
    /// Both halves of one. Asked to be the **compile server** it writes down
    /// what its own sandbox can reach and then sits there holding it, which is
    /// what a real sccache does and the only thing about it worth asserting:
    /// the paths it is checked against are this fixture's, so they are written
    /// into the script rather than looked for in an environment that is closed
    /// by the time it runs. Asked anything else it is a session's client, and
    /// says which build it is — see [`SAYS_WHICH_SCCACHE`].
    fn sccache(&self) -> PathBuf {
        let path = self.state.path().join("sccache");
        let settings = Settings::in_data_dir(self.state.path());

        let script = format!(
            r#"#!/bin/sh
{PROBE}
if [ "${{SCCACHE_START_SERVER-}}" = "1" ]; then
    {{
        say home "$HOME"
        say no-daemon "${{SCCACHE_NO_DAEMON-unset}}"
        say idle "${{SCCACHE_IDLE_TIMEOUT-unset}}"
        say size "${{SCCACHE_CACHE_SIZE-unset}}"
        say sccache-dir "${{SCCACHE_DIR-unset}}"
        dir {worktrees} worktrees
        dir {cache} cache
        dir {handoffs} handoffs
        dir {attachments} attachments
        file {database} database
        file {config} config
        file {secrets} secrets
    }} > {report}
    # Held open, because what is being asserted is a server that is *there* —
    # and bounded, because nothing in the test stops it.
    sleep 30
    exit 0
fi
{SAYS_WHICH_SCCACHE}"#,
            worktrees = quoted(&self.worktrees_dir()),
            cache = quoted(&self.cache_dir()),
            handoffs = quoted(&self.state.path().join("handoffs")),
            attachments = quoted(&self.state.path().join("attachments")),
            database = quoted(&self.state.path().join("verkstead.db")),
            config = quoted(&settings.config_path()),
            secrets = quoted(&settings.secrets_path()),
            report = quoted(&self.cache_dir().join(COMPILE_SERVER_REPORT)),
        );

        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

        path
    }

    /// Put a file in the Conversation's attachments directory, the way an
    /// upload does.
    ///
    /// Written rather than uploaded, because what is being probed is the bind: a
    /// route, a body limit and a row about the file are the attaching tests'
    /// half, and what a sandbox reaches for is a directory with something in it.
    /// Where that directory is, is [`verkstead_server::attachments`]'s own and
    /// its own tests' to hold.
    fn attach(&self, name: &str, body: &[u8]) {
        let directory = self
            .state
            .path()
            .join("attachments")
            .join(self.conversation.id.to_string());

        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(name), body).unwrap();
    }

    /// The companion of that name, as the Conversation now carries it — where
    /// it was checked out, and what it holds.
    fn companion(&self, name: &str) -> &store::Companion {
        self.conversation
            .companions
            .iter()
            .find(|companion| companion.repo.name == name)
            .unwrap_or_else(|| panic!("the fixture added {name} as a companion"))
    }

    /// And where it was checked out, which is what a session is given.
    fn companion_worktree(&self, name: &str) -> &Path {
        self.companion(name)
            .worktree
            .as_deref()
            .expect("a grilling Conversation's companions are checked out")
    }

    /// The same, for a server that is really listening somewhere — which is what
    /// a session inside has to be able to reach to ask anything.
    fn sandbox_reaching(
        &self,
        listening: SocketAddr,
        cache: &BuildCache,
        extra: Vec<Bind>,
    ) -> Sandbox {
        self.sandbox_under(&self.profile, listening, cache, extra)
    }

    /// And one around a Profile that is not the fixture's own, which is how the
    /// second agent type gets a sandbox to be probed inside.
    fn sandbox_under(
        &self,
        profile: &store::Profile,
        listening: SocketAddr,
        cache: &BuildCache,
        extra: Vec<Bind>,
    ) -> Sandbox {
        Sandbox::for_conversation(
            &self.conversation,
            profile,
            &self.homes(),
            &Reachable::at(listening),
            &self.skills,
            &self.verkstead,
            &self.handoffs,
            &self.attachments,
            // Read here rather than at startup, which is where the server reads
            // them too: a sandbox carries the token and the author that were
            // configured when it was built.
            &self.settings.secrets(),
            &self.settings.config(),
            cache,
            extra,
        )
        .expect("a grilling Conversation has a worktree to build a sandbox around")
    }

    /// And one built the way `platform` builds one, which is how the open
    /// rendering is reached from a machine that is not the one it is for — see
    /// [`the_open_rendering_hands_a_session_the_environment_it_was_described_with`].
    fn sandbox_on(&self, platform: Platform) -> Sandbox {
        self.sandbox_on_under(&self.profile, platform)
            .expect("a grilling Conversation has a worktree to build a sandbox around")
    }

    /// The same, around a Profile that is not the fixture's own — and handing
    /// back what it really answered, which is how the one Conversation that
    /// cannot have a sandbox at all is asked about.
    ///
    /// **The Skills and the executable are built for `platform` too**, rather
    /// than reused off the fixture. Where a session finds either of them is
    /// that platform's answer — see `Skills::installed` and `Executable::at` —
    /// and the fixture's own were built for the machine this is running on. A
    /// description mixing the two is one where the rendering that joins a path
    /// in by hand is asked to clear and link a name off the *host's* root,
    /// which is neither what a Windows session would find nor a thing to do to
    /// the machine running the suite.
    fn sandbox_on_under(&self, profile: &store::Profile, platform: Platform) -> Option<Sandbox> {
        Sandbox::for_conversation(
            &self.conversation,
            profile,
            &Homes::on(platform, self.home.path().to_owned(), self.state.path()),
            &Reachable::at(LISTENING),
            &Skills::installed(platform, self.state.path()).expect("this binary carries skills"),
            &Executable::at(
                platform,
                self.verkstead.path().to_owned(),
                self.state.path(),
            )
            .expect("the fixture's image is still there"),
            &self.handoffs,
            &self.attachments,
            &self.settings.secrets(),
            &self.settings.config(),
            &BuildCache::none(),
            vec![],
        )
    }

    /// Where this Conversation's own Windows profile is: under the Data
    /// Directory, named for the Conversation, and made fresh as each of its
    /// sessions is rendered — see [`verkstead_server::sandbox::Homes`].
    fn windows_profile(&self) -> PathBuf {
        self.state
            .path()
            .join("homes")
            .join(self.conversation.id.to_string())
    }

    /// What the fixture's own Claude Profile names: the directory half of the
    /// account, and the file half.
    fn claude_dir(&self) -> PathBuf {
        self.elsewhere.path().join("account/.claude")
    }

    fn claude_config(&self) -> PathBuf {
        self.elsewhere.path().join("account/.claude.json")
    }

    /// And the directory the Codex Profile names.
    fn codex_dir(&self) -> PathBuf {
        self.elsewhere.path().join("codex-account/.codex")
    }

    /// And the directory the Grok Build Profile names.
    fn grok_dir(&self) -> PathBuf {
        self.elsewhere.path().join("grok-account/.grok")
    }

    /// And the home the OpenCode Profile names.
    fn opencode_home(&self) -> PathBuf {
        self.elsewhere.path().join("opencode-account/opencode")
    }

    /// Write `secrets.yaml` as the settings page would, so that the sandboxes
    /// built after this carry the token.
    fn configure_github_token(&self, yaml: &str) {
        std::fs::write(self.settings.secrets_path(), yaml).unwrap();
    }

    /// And `config.yaml`, which is who those sandboxes commit as and how their
    /// shared build cache is set.
    fn configure(&self, yaml: &str) {
        std::fs::write(self.settings.config_path(), yaml).unwrap();
    }

    /// A Profile of the second agent type, whose whole account is one home.
    ///
    /// Saved into the same store the fixture's own was, with a home beside the
    /// repository holding what a Codex account that has been used holds: a
    /// login, a configuration naming a provider of its own beside the human's
    /// MCP servers, a rollout and a memory, and the human's own rules, skills,
    /// instructions, history, archive and databases — so that what a session's
    /// root leaves out is a claim about files that are there.
    async fn codex_profile(&self) -> store::Profile {
        let home = self.codex_dir();

        for dir in [
            "sessions/2026/09/01",
            "memories",
            "rules",
            "skills/the-accounts-own",
            "archived_sessions",
        ] {
            std::fs::create_dir_all(home.join(dir)).unwrap();
        }

        for (file, contents) in [
            ("auth.json", "{\"the\": \"login\"}\n"),
            ("config.toml", CODEX_ACCOUNT_CONFIG),
            ("sessions/2026/09/01/rollout-the-humans.jsonl", "{}\n"),
            ("memories/MEMORY.md", "remembered\n"),
            ("rules/default.rules", "# the human's\n"),
            ("skills/the-accounts-own/SKILL.md", "# the human's\n"),
            ("AGENTS.md", "# the human's\n"),
            ("history.jsonl", "{}\n"),
            ("state_5.sqlite", "not really a database\n"),
        ] {
            std::fs::write(home.join(file), contents).unwrap();
        }

        store::create_profile(
            &self.pool,
            &store::ProfileFacts {
                name: Some("codex".to_owned()),
                account: store::Account::Codex { home },
                models: vec!["gpt-5-codex".to_owned()],
                memory: true,
            },
        )
        .await
        .unwrap()
        .expect("nothing is called that yet")
    }

    /// And one of the third, whose account is one home as the second's is.
    ///
    /// Holding what a Grok Build 1.0.13 account that has been used holds: a
    /// login, a configuration reaching a model of its own beside the human's MCP
    /// servers, a session's log and the search database beside it, the global
    /// memory and a repository's with its index, and the human's own skills,
    /// plugins, interface settings and logs — so that what a session's root
    /// leaves out is a claim about files that are there.
    async fn grok_profile(&self) -> store::Profile {
        let home = self.grok_dir();

        for dir in [
            "sessions/%2Fsrc%2Fthe-humans/019-the-humans",
            "memory/the-humans-0123abcd",
            "skills/the-accounts-own",
            "plugins",
            "logs",
        ] {
            std::fs::create_dir_all(home.join(dir)).unwrap();
        }

        for (file, contents) in [
            ("auth.json", GROK_LOGIN),
            ("config.toml", GROK_ACCOUNT_CONFIG),
            (
                "sessions/%2Fsrc%2Fthe-humans/019-the-humans/updates.jsonl",
                "{}\n",
            ),
            ("sessions/session_search.sqlite", "not really a database\n"),
            ("memory/MEMORY.md", "remembered\n"),
            (
                "memory/the-humans-0123abcd/index.sqlite",
                "not really a database\n",
            ),
            (
                "skills/the-accounts-own/SKILL.md",
                "# what grok found there\n",
            ),
            ("pager.toml", "# the human's\n"),
            ("logs/unified.jsonl", "{}\n"),
        ] {
            std::fs::write(home.join(file), contents).unwrap();
        }

        store::create_profile(
            &self.pool,
            &store::ProfileFacts {
                name: Some("grok".to_owned()),
                account: store::Account::Grok { home },
                models: vec!["grok-4.6".to_owned()],
                memory: true,
            },
        )
        .await
        .unwrap()
        .expect("nothing is called that yet")
    }

    /// And one of the fourth, whose account is one home holding the two
    /// directories opencode keeps an account in.
    ///
    /// Made the way a human makes one — a `HOME=<it> opencode` run leaves
    /// exactly these — with something inside each so the test can say which
    /// landed where. The config directory holds the human's own skills,
    /// plugins and an `opencode.jsonc` with comments; the data directory holds
    /// the login and a database with its write-ahead-log siblings.
    async fn opencode_profile(&self) -> store::Profile {
        let home = self.opencode_home();
        let config = home.join(".config/opencode");
        let data = home.join(".local/share/opencode");

        std::fs::create_dir_all(config.join("skills/the-accounts-own")).unwrap();
        std::fs::write(
            config.join("skills/the-accounts-own/SKILL.md"),
            "# what opencode found there\n",
        )
        .unwrap();
        std::fs::create_dir_all(config.join("node_modules")).unwrap();
        std::fs::write(config.join("package.json"), "{}\n").unwrap();
        std::fs::write(config.join("opencode.jsonc"), OPENCODE_ACCOUNT_CONFIG).unwrap();

        std::fs::create_dir_all(&data).unwrap();
        for (file, contents) in [
            ("auth.json", OPENCODE_LOGIN),
            ("opencode.db", "the human's sessions\n"),
            ("opencode.db-wal", "\n"),
            ("opencode.db-shm", "\n"),
            ("mcp-auth.json", "{}\n"),
        ] {
            std::fs::write(data.join(file), contents).unwrap();
        }

        store::create_profile(
            &self.pool,
            &store::ProfileFacts {
                name: Some("opencode".to_owned()),
                account: store::Account::OpenCode { home },
                models: vec!["opencode/big-pickle".to_owned()],
                memory: true,
            },
        )
        .await
        .unwrap()
        .expect("nothing is called that yet")
    }

    /// The host home a sandbox is built around — the fixture's rather than
    /// whoever is running the tests, so what `~` holds is decided here.
    fn homes(&self) -> Homes {
        Homes::on(
            Platform::HERE,
            self.home.path().to_owned(),
            self.state.path(),
        )
    }

    /// Where `~` is, as the probe will see it.
    fn home_path(&self) -> &Path {
        self.home.path()
    }

    fn worktree(&self) -> &Path {
        self.conversation
            .worktree
            .as_deref()
            .expect("a grilling Conversation has a worktree")
    }
}

/// Stand one up, with no companion repos — which is the ordinary Conversation
/// and what most of these tests are about.
async fn grilling() -> Grilling {
    grilling_alongside(&[]).await
}

/// And one configured with companion repos, each registered under the name given
/// and added in the mode given.
///
/// They are added while the Conversation is still drafting, which is the only
/// time they can be, and then checked out beside its own the way a grill start
/// checks them out: a read-write companion on a branch of its own, a read-only
/// one detached at the commit its base resolved to.
async fn grilling_alongside(companions: &[(&str, store::CompanionMode)]) -> Grilling {
    let elsewhere = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // A gitconfig of the machine's, which sandboxes used to be given and are
    // not: who a session commits as is configured now, so this is here to be
    // absent from inside rather than to be found there.
    std::fs::write(
        home.path().join(".gitconfig"),
        "[user]\n\tname = Whoever The Host Is\n\temail = host@verkstead.invalid\n",
    )
    .unwrap();

    // And a gh login of the machine's, in both the places gh keeps one: no
    // session has any business seeing either, whatever the host is logged in as.
    for config in [".config/gh", ".xdg-config/gh"] {
        std::fs::create_dir_all(home.path().join(config)).unwrap();
        std::fs::write(
            home.path().join(config).join("hosts.yml"),
            "github.com:\n    user: nobody\n",
        )
        .unwrap();
    }

    // Something in HOME that is none of the sandbox's business, so that "the
    // rest of HOME is absent" is a claim about this run rather than about an
    // empty directory.
    std::fs::write(home.path().join(".bash_history"), "rm -rf /\n").unwrap();
    std::fs::create_dir_all(home.path().join(".ssh")).unwrap();
    std::fs::write(home.path().join(".ssh/id_ed25519"), "a private key\n").unwrap();

    // The skills the host keeps for its own agents, in the checkout every one of
    // these sessions used to be given. Verkstead ships its own now, and this is
    // here so that "no `~/src/tobico-skills`" is a claim about a directory that
    // exists on the host rather than about one nobody made.
    std::fs::create_dir_all(home.path().join("src/tobico-skills/skills/grilling")).unwrap();
    std::fs::write(
        home.path()
            .join("src/tobico-skills/skills/grilling/SKILL.md"),
        "# the host's own\n",
    )
    .unwrap();

    let repo = repository(elsewhere.path().join("verkstead"));
    let sibling = repository(elsewhere.path().join("something-else"));

    let pool = store::open_database(&state.path().join("verkstead.db"))
        .await
        .unwrap();

    let repo_row = store::register_repo(&pool, &repo, "verkstead", "main")
        .await
        .unwrap()
        .expect("the Repo registers");

    let claude_dir = elsewhere.path().join("account/.claude");
    let config_file = elsewhere.path().join("account/.claude.json");
    std::fs::create_dir_all(&claude_dir).unwrap();
    std::fs::write(claude_dir.join("settings.json"), "{}\n").unwrap();
    std::fs::write(&config_file, "{}\n").unwrap();

    // And a skill of the account's own, where a Claude session would otherwise
    // go looking: what a session is grilled by is the product's, so this is what
    // being hidden looks like from inside.
    std::fs::create_dir_all(claude_dir.join("skills/the-accounts-own")).unwrap();
    std::fs::write(
        claude_dir.join("skills/the-accounts-own/SKILL.md"),
        "# the account's own\n",
    )
    .unwrap();

    // And the rest of what a human's account holds that a session is not given:
    // a login, which it is, beside plugins, a global CLAUDE.md, a history and
    // another repository's transcripts, which it is not.
    std::fs::write(
        claude_dir.join(".credentials.json"),
        "{\"the\": \"login\"}\n",
    )
    .unwrap();
    std::fs::create_dir_all(claude_dir.join("plugins/the-accounts-own")).unwrap();
    std::fs::write(claude_dir.join("CLAUDE.md"), "# the human's own\n").unwrap();
    std::fs::write(claude_dir.join("history.jsonl"), "{}\n").unwrap();
    std::fs::create_dir_all(claude_dir.join("projects/-home-you-src-something-else")).unwrap();
    std::fs::write(
        claude_dir.join("projects/-home-you-src-something-else/another.jsonl"),
        "{}\n",
    )
    .unwrap();

    let profile = store::create_profile(
        &pool,
        &store::ProfileFacts {
            name: Some("work".to_owned()),
            account: store::Account::Claude {
                claude_dir,
                config_file,
            },
            models: vec!["claude-opus-5".to_owned()],
            memory: true,
        },
    )
    .await
    .unwrap()
    .expect("the Profile saves");

    let id = store::start_conversation(&pool, repo_row.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Conversation starts");

    store::set_grilling_pairing(&pool, id, profile.id, profile.model())
        .await
        .unwrap();
    store::set_implementation_pairing(&pool, id, profile.id, profile.model())
        .await
        .unwrap();

    // The worktree git itself made, where the server puts one.
    let worktree = state.path().join("worktrees/verkstead-rate-limiting");
    std::fs::create_dir_all(worktree.parent().unwrap()).unwrap();
    let commit = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-b",
            "rate-limiting",
            &worktree.to_string_lossy(),
            &commit,
        ],
    );

    // And one beside it per companion the test asked for, in the shape its mode
    // gives it — see [`grilling_alongside`].
    let mut checkouts = Vec::new();

    for (name, mode) in companions {
        let path = repository(elsewhere.path().join(name));
        let registered = store::register_repo(&pool, &path, name, "main")
            .await
            .unwrap()
            .expect("the companion Repo registers");

        assert_eq!(
            store::add_companion(&pool, id, registered.id)
                .await
                .unwrap(),
            store::Adding::Added,
        );
        assert_eq!(
            store::configure_companion(&pool, id, registered.id, store::Change::Mode(*mode))
                .await
                .unwrap(),
            store::Configured::Saved,
        );

        let at = git(&path, &["rev-parse", "HEAD"]).trim().to_owned();

        // Named for the Repo and what the checkout holds, as the real one is:
        // the branch where there is one, and the base it stands at where there
        // is not.
        let checkout = match mode {
            store::CompanionMode::ReadOnly => {
                let checkout = state.path().join(format!("worktrees/{name}-main"));
                git(
                    &path,
                    &[
                        "worktree",
                        "add",
                        "--detach",
                        &checkout.to_string_lossy(),
                        &at,
                    ],
                );
                checkout
            }
            store::CompanionMode::ReadWrite => {
                let checkout = state.path().join(format!("worktrees/{name}-rate-limiting"));
                git(
                    &path,
                    &[
                        "worktree",
                        "add",
                        "-b",
                        "rate-limiting",
                        &checkout.to_string_lossy(),
                        &at,
                    ],
                );
                checkout
            }
        };

        checkouts.push(store::CompanionWorktree {
            repo_id: registered.id,
            path: checkout,
            base_commit: Some(at),
        });
    }

    store::start_grilling(&pool, id, &commit, &worktree, &checkouts)
        .await
        .unwrap();

    let conversation = store::load_conversation(&pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there");

    let skills =
        Skills::installed(Platform::HERE, state.path()).expect("this binary carries skills");
    let handoffs = Handoffs::under(state.path());
    let attachments = Attachments::under(state.path());
    let settings = Settings::in_data_dir(state.path());

    // The executable a session is equipped with, somewhere no session can reach
    // it except through the bind — see [`SAYS_WHICH_BUILD`].
    // Not under `bin/`, which is where a platform with no binds to make one
    // with puts what a session finds: the two are different places, and a
    // fixture that stood the image in the second of them would be proving
    // nothing about how it got there.
    let image = state.path().join("image/verkstead");
    std::fs::create_dir_all(image.parent().unwrap()).unwrap();
    std::fs::write(&image, SAYS_WHICH_BUILD).unwrap();
    std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o755)).unwrap();
    let verkstead = Executable::at(Platform::HERE, image, state.path())
        .expect("the executable was just written");

    Grilling {
        elsewhere,
        state,
        home,
        repo,
        sibling,
        conversation,
        profile,
        pool,
        skills,
        verkstead,
        handoffs,
        attachments,
        settings,
    }
}

/// A git repository at `path`, with one commit on `main` and a GitHub remote it
/// was cloned over SSH from.
///
/// Both of those are what the sandbox has to be able to override. The local
/// identity is the one a repository happens to carry, and the SSH remote is the
/// one there are no keys inside a sandbox for — see
/// [`an_ssh_github_remote_resolves_to_https_and_the_token_is_what_pushes_it`].
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "local@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Whatever The Repo Says"]);
    git(
        &path,
        &[
            "remote",
            "add",
            "origin",
            "git@github.com:tobico/verkstead.git",
        ],
    );
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    git(&path, &["add", "README.md"]);
    git(&path, &["commit", "-m", "first"]);

    path
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect("git should be on the PATH for these tests");

    assert!(
        output.status.success(),
        "git {args:?} failed in {}",
        dir.display()
    );

    String::from_utf8(output.stdout).unwrap()
}

/// The shell the probe is written in, and the one path in the sandbox that is
/// guaranteed to hold one: `/bin/sh` is what the system bind puts there.
const SH: &str = "/bin/sh";

/// What the probe says about one path.
///
/// Everything the sandbox does *not* bind reads as `absent`, which is the whole
/// of what "everything else in HOME simply absent" means from inside.
const PROBE: &str = r#"
say() { printf '%s=%s\n' "$1" "$2"; }

dir() {
    if [ ! -d "$1" ]; then say "$2" absent; return; fi
    if (exec 3> "$1/.verkstead-probe") 2>/dev/null; then
        rm -f "$1/.verkstead-probe"
        say "$2" write
    elif ls "$1" >/dev/null 2>&1; then
        say "$2" read
    else
        say "$2" hidden
    fi
}

file() {
    if [ ! -f "$1" ]; then say "$2" absent; return; fi
    # Opening for append is the write, and it changes not a byte of what is
    # there. In a subshell, because a redirection that fails takes the shell
    # attempting it down with it.
    if (exec 3>> "$1") 2>/dev/null; then
        say "$2" write
    elif cat "$1" >/dev/null 2>&1; then
        say "$2" read
    else
        say "$2" hidden
    fi
}
"#;

/// Run `script` inside `sandbox` and read back what it reported.
fn probe(sandbox: &Sandbox, script: &str) -> BTreeMap<String, String> {
    probe_closing(sandbox, script).0
}

/// The same, keeping what the session's ending is left to see to — which is
/// what the tests about a login written inside have to close themselves.
fn probe_closing(sandbox: &Sandbox, script: &str) -> (BTreeMap<String, String>, Closing) {
    let whole = format!("{PROBE}\n{script}\n");

    let (rendering, closing) = sandbox
        .command(&[SH, "-c", &whole])
        .expect("a rendering on a platform with no identity to make");

    let output = Command::try_from(&rendering)
        .expect("a rendering with no container")
        .stdin(Stdio::null())
        .output()
        .expect("bwrap should be on the PATH: the dev shell declares bubblewrap");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the probe failed inside the sandbox: {stderr}"
    );

    let reported = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();

    (reported, closing)
}

/// And what the process a sandbox renders to is actually handed, read off the
/// machine's own `env` run with nothing in front of it.
///
/// Not the probe above, which is a shell script and therefore a shell: what is
/// being asked here is what the *first* thing started inside gets, and a shell
/// between it and the rendering is a shell that says something of its own.
fn environment(sandbox: &Sandbox) -> BTreeMap<String, String> {
    let rendered = sandbox
        .command(&[on_the_host("env")])
        .expect("a rendering on a platform with no identity to make");

    let output = Command::try_from(&rendered.0)
        .expect("a rendering with no container")
        .stdin(Stdio::null())
        .output()
        .expect("the rendering to be startable");

    assert!(
        output.status.success(),
        "the probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// Where a program is on the host, absolute.
///
/// The probe calls the few tools it needs by their whole path, because the
/// sandbox's `PATH` is the machine's system profile rather than the shell the
/// tests were started from — and everything under `/nix` is reachable inside
/// either way.
fn on_the_host(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("{program} should be on the PATH for these tests"))
}

#[tokio::test]
async fn the_worktree_the_git_directory_and_the_handoff_directory_are_what_can_be_written() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {worktree} worktree
            dir {git_dir} git-dir
            dir /tmp/verkstead handoff
            "#,
            worktree = quoted(fixture.worktree()),
            git_dir = quoted(&fixture.repo.join(".git")),
        ),
    );

    assert_eq!(reported["worktree"], "write", "a session commits its work");
    assert_eq!(
        reported["git-dir"], "write",
        "the objects and refs a commit is written into are the Repo's, not the worktree's"
    );
    assert_eq!(
        reported["handoff"], "write",
        "and the Conversation's own directory, which is the one writable place git will never see"
    );
}

/// The handoff directory is a bind and not part of the tmpfs `/tmp` is
/// otherwise made of — which is the whole of what makes it useful: a document
/// written in there has to be one Verkstead can read once the session is gone.
#[tokio::test]
async fn what_a_session_writes_in_its_handoff_directory_is_there_when_it_has_gone() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        r#"
        printf '# What we settled\n' > /tmp/verkstead/handoff.md
        say wrote yes
        "#,
    );

    assert_eq!(reported["wrote"], "yes");

    let outside = fixture
        .handoffs
        .directory(fixture.conversation.id)
        .expect("the directory the sandbox bound")
        .join("handoff.md");

    assert_eq!(
        std::fs::read_to_string(&outside).ok().as_deref(),
        Some("# What we settled\n"),
        "nothing written inside reached {}",
        outside.display()
    );
}

/// The other half of the Conversation's own directory outside the worktree: the
/// files the human attached, at the path the prompt names them at.
///
/// **Read-only**, which `dir` reporting `read` is the whole of: the listing
/// worked and a file could not be created beside what is there. The copy is the
/// record, and an agent that wants to work on a file copies it into the
/// Worktree.
#[tokio::test]
async fn the_attached_files_are_read_at_the_path_the_prompt_names_and_written_nowhere() {
    let fixture = grilling().await;
    fixture.attach("wireframe.png", b"PNG");

    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        // The path is written out rather than asked of the server, for the
        // reason the handoff directory's is above: what a session opens is a
        // path it read in its prompt, and a test that composed the same path
        // twice would agree with itself about it.
        r#"
        dir /verkstead/attachments attachments
        file /verkstead/attachments/wireframe.png attached
        "#,
    );

    assert_eq!(
        reported["attachments"], "read",
        "a session reads what was attached and cannot add anything beside it"
    );
    assert_eq!(
        reported["attached"], "read",
        "nor write over the file the human handed over"
    );
}

/// And a Conversation nothing was attached to is bound over an empty directory
/// all the same, so a session that blocked on an ask can read a file put on an
/// Answer while it waited.
///
/// The sandbox is composed before the file exists, which is the whole of what
/// this asserts: the bind is decided as a session is started, and a session
/// blocked on an ask was started hours before the human answered it. Nothing
/// says the path is there — the prompt lists the files there are, and a
/// Conversation with none is told nothing at all.
#[tokio::test]
async fn a_file_attached_after_a_session_started_is_read_at_that_path_too() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    fixture.attach("rates.csv", b"1,2,3");

    let reported = probe(
        &sandbox,
        r#"
        dir /verkstead/attachments attachments
        file /verkstead/attachments/rates.csv attached
        "#,
    );

    assert_eq!(
        reported["attachments"], "read",
        "the directory was bound though it was empty when the session started"
    );
    assert_eq!(
        reported["attached"], "read",
        "and the file the Response names is there to be read"
    );
}

#[tokio::test]
async fn the_system_comes_in_read_only_and_the_hosts_gitconfig_not_at_all() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir /nix nix
            file {gitconfig} gitconfig
            "#,
            gitconfig = quoted(&fixture.home_path().join(".gitconfig")),
        ),
    );

    assert_eq!(reported["nix"], "read");
    assert_eq!(
        reported["gitconfig"], "absent",
        "who a session commits as is configured rather than found lying about in a home directory"
    );
}

/// Who a session commits as: the configured author, proved by a commit made
/// inside and read back outside.
///
/// The repository has an identity of its own and the host has a gitconfig, and
/// neither is what lands on the commit — which is the point of configuring one
/// at all. A session works in a checkout somebody else made, and what it commits
/// as should be a fact about the installation rather than about whatever that
/// checkout was left holding.
#[tokio::test]
async fn a_commit_made_inside_is_by_the_configured_author() {
    let fixture = grilling().await;
    fixture.configure("git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n");

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            if {git} commit --quiet --allow-empty -m 'from inside' 2>/tmp/git-said; then
                say committed yes
            else
                say committed "no: $(cat /tmp/git-said)"
            fi
            "#,
            git = quoted(&on_the_host("git")),
        ),
    );

    assert_eq!(reported["committed"], "yes");

    // Read outside, off the branch the worktree is on: the commit is in the
    // Repo's object database, which is what the session was given to write into.
    assert_eq!(
        git(fixture.worktree(), &["log", "-1", "--format=%an <%ae>"]).trim(),
        "Tobias Cohen <tobi@tobico.net>",
        "the commit is by whoever config.yaml says, not by the repository's own \
         local identity and not by the host's gitconfig"
    );
}

/// And with nobody configured, git's own refusal stands. No author is invented
/// — a commit by `verkstead@localhost` is the one nobody notices — and the
/// settings page is where the missing state gets surfaced.
///
/// The repository's own identity is taken away first, because a checkout that
/// carries one is not the case being asked about: what has to be true is that a
/// session with nothing to commit as says so rather than committing as
/// something.
#[tokio::test]
async fn no_author_configured_is_git_asking_to_be_told_who_you_are() {
    let fixture = grilling().await;

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            {git} config --unset user.name
            {git} config --unset user.email

            if {git} commit --quiet --allow-empty -m 'from inside' 2>/tmp/git-said; then
                say committed yes
            else
                say committed no
            fi

            say said "$({grep} -c 'tell me who you are' /tmp/git-said)"
            "#,
            git = quoted(&on_the_host("git")),
            grep = quoted(&on_the_host("grep")),
        ),
    );

    assert_eq!(
        reported["committed"], "no",
        "with nobody configured anywhere there is nobody to commit as"
    );
    assert_eq!(
        reported["said"], "1",
        "and what a session is left with is git's own answer, which says what to configure"
    );
}

/// A push out of a sandbox goes over HTTPS with the token, whatever the remote
/// the repository was cloned from says.
///
/// There are no SSH keys inside a sandbox and there is not going to be one: the
/// credentials are the token, and an SSH remote would fail on a key that is not
/// there rather than fall back to anything. So the URL is rewritten as git
/// resolves it — the repository's own `.git/config` is left saying exactly what
/// the human cloned — and the credential helper is `gh`'s, which answers out of
/// `GH_TOKEN`.
///
/// Asked of git inside rather than of the flags, like everything else here: what
/// settles it is git resolving the remote and naming the helper it would ask.
#[tokio::test]
async fn an_ssh_github_remote_resolves_to_https_and_the_token_is_what_pushes_it() {
    let fixture = grilling().await;
    fixture.configure_github_token("github_token: ghp_theconfiguredone\n");

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            say remote "$({git} ls-remote --get-url origin)"
            say written-down "$({git} config --get remote.origin.url)"
            say helper "$({git} config --get-urlmatch credential.helper https://github.com)"
            say token "${{GH_TOKEN-unset}}"
            say prompt "${{GIT_TERMINAL_PROMPT-unset}}"
            "#,
            git = quoted(&on_the_host("git")),
        ),
    );

    assert_eq!(
        reported["remote"], "https://github.com/tobico/verkstead.git",
        "an SSH remote is resolved to the HTTPS one the token is any use for"
    );
    assert_eq!(
        reported["written-down"], "git@github.com:tobico/verkstead.git",
        "and the repository still says what the human cloned"
    );
    assert_eq!(
        reported["helper"], "!gh auth git-credential",
        "which is what turns GH_TOKEN into an authenticated push"
    );
    assert_eq!(reported["token"], "ghp_theconfiguredone");
    assert_eq!(
        reported["prompt"], "0",
        "and a push that cannot authenticate says so rather than asking a terminal \
         nobody is sitting at"
    );
}

/// The other spelling of the same remote, which a `.gitmodules` or an older
/// clone is as likely to hold.
#[tokio::test]
async fn the_url_form_of_an_ssh_github_remote_is_rewritten_too() {
    let fixture = grilling().await;

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            {git} remote set-url origin ssh://git@github.com/tobico/verkstead.git
            say remote "$({git} ls-remote --get-url origin)"
            "#,
            git = quoted(&on_the_host("git")),
        ),
    );

    assert_eq!(
        reported["remote"], "https://github.com/tobico/verkstead.git",
        "`ssh://git@github.com/` is the same remote written another way"
    );
}

/// The third rendering, asked here rather than in a suite of its own: what a
/// session on the platform with no boundary yet is handed.
///
/// **The rendering is portable and the boundary is what is not**, which is why
/// this can be asked on the machine running these tests at all. There are no
/// flags and no policy in it — it sets the environment, starts in the
/// Conversation's Worktree and runs the vector — so a Conversation built
/// against a Windows [`Homes`] renders to a process this machine can start and
/// read back. What that cannot say is what a Windows machine makes of it, which
/// is the Windows job's to say; what it does say is the whole of the
/// description, which is what a rendering is.
///
/// The probe is the machine's own `env`, run with nothing in front of it: a
/// wrapper would be the first thing in the report.
#[tokio::test]
async fn the_open_rendering_hands_a_session_the_environment_it_was_described_with() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox_on(Platform::Windows);

    let reported = environment(&sandbox);

    // Counted by its own variable rather than written down here: how many
    // pairs git's configuration comes to is that configuration's business.
    let git_config: usize = reported["GIT_CONFIG_COUNT"].parse().expect("a count");

    let mut expected: BTreeSet<String> = [
        "HOME",
        "PATH",
        "SHELL",
        "TERM",
        "VERKSTEAD_SERVER",
        "VERKSTEAD_AGENT",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "TEMP",
        "TMP",
        "GIT_CONFIG_COUNT",
        "GIT_TERMINAL_PROMPT",
        // This fixture's Profile is a Claude one, and every Claude session is
        // told not to update the install it is running — see
        // `a_claude_session_is_told_not_to_update_the_install_it_is_running`.
        "DISABLE_AUTOUPDATER",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain((0..git_config).flat_map(|n| {
        [
            format!("GIT_CONFIG_KEY_{n}"),
            format!("GIT_CONFIG_VALUE_{n}"),
        ]
    }))
    .collect();

    // And whichever of the machine's own names the machine has. None of them on
    // the Linux this suite runs on, which is what makes the set above the whole
    // of it here — they are read off the server's environment, and a Windows
    // machine is the only one that has them.
    expected.extend(
        ["SystemRoot", "SystemDrive", "ComSpec", "PATHEXT"]
            .into_iter()
            .filter(|name| std::env::var_os(name).is_some())
            .map(str::to_owned),
    );

    assert_eq!(
        reported.keys().cloned().collect::<BTreeSet<String>>(),
        expected,
        "what a session has is what the description said and nothing else — \
         the environment is cleared, so the harness's own is not in it"
    );

    assert_eq!(
        reported["HOME"],
        fixture.windows_profile().display().to_string(),
        "and the one name both platforms read is the profile Verkstead made for \
         this Conversation rather than the server's own"
    );
    // Which leads with where the running image really is, this being the
    // platform that binds nothing and links nothing: what a session asks with
    // is the build serving it, said as the path that build is at rather than as
    // a name a mount would have made. See `Executable::at`.
    assert_eq!(
        reported["PATH"],
        format!(
            "{};{}",
            fixture.state.path().join("image").display(),
            std::env::var("PATH").expect("this machine has a PATH")
        ),
        "Verkstead's own directory leads, and what follows it is where the human \
         on this machine put their tools"
    );

    // The five that follow HOME on this platform, which is where a Windows
    // program looks for the account's own things — see the sandbox module.
    let profile = Path::new(&reported["USERPROFILE"]);

    assert_eq!(profile, fixture.windows_profile());
    assert_eq!(
        Path::new(&reported["APPDATA"]),
        profile.join("AppData").join("Roaming")
    );
    assert_eq!(
        Path::new(&reported["LOCALAPPDATA"]),
        profile.join("AppData").join("Local")
    );
    assert_eq!(reported["TEMP"], reported["TMP"]);
    assert!(
        Path::new(&reported["TEMP"]).starts_with(profile),
        "a file a session throws away should land under its own profile, and \
         TEMP is {}",
        reported["TEMP"]
    );
}

/// The profile that rendering makes, and the Profile's account joined into it.
///
/// **Asked of the description rather than of a session**, for the reason the
/// environment above is asked that way: what this rendering makes, it makes on
/// whichever machine renders it. There is no boundary on that platform, so
/// every path here is a host path and every one of them can be read from
/// outside — which is the whole of what a session inside would find. The one
/// word that differs is the mechanism: a directory is joined in by a junction
/// there and by a symbolic link on the machine running this, and it is the
/// `windows-2025` job that reads the first of those back.
///
/// Claude's, which is a root of Verkstead's own with the login hard-linked into
/// it and this Repo's and this Worktree's `projects/` entries junctioned in —
/// and beside it the file half of the pair, hard-linked as it always was.
#[tokio::test]
async fn a_windows_session_finds_the_profiles_account_inside_a_profile_of_its_own() {
    let fixture = grilling().await;

    made(&fixture.sandbox_on(Platform::Windows));

    let profile = fixture.windows_profile();
    let root = profile.join(".claude");

    assert!(
        !std::fs::symlink_metadata(&root).unwrap().is_symlink(),
        "the root is a directory of Verkstead's own, not a junction to the account"
    );

    let mut held: Vec<String> = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    held.sort();

    assert_eq!(
        held,
        [".credentials.json", "projects", "settings.json"],
        "the login, the projects directory and the settings Verkstead wrote are \
         the whole of the root: none of the account's plugins, skills, \
         `CLAUDE.md` or history"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("settings.json")).unwrap(),
        "{\n  \"skipDangerousModePermissionPrompt\": true\n}\n",
        "and the settings are Verkstead's, holding the bypass key and nothing of \
         an account whose own settings carry nothing over"
    );

    let mut entries: Vec<String> = std::fs::read_dir(root.join("projects"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();

    let mut expected = vec![entry_named(&fixture.repo), entry_named(fixture.worktree())];
    expected.sort();

    assert_eq!(
        entries, expected,
        "and under it this Repo's entry and this Worktree's, and no other \
         repository's"
    );

    assert_eq!(
        std::fs::read_to_string(root.join(".credentials.json")).unwrap(),
        "{\"the\": \"login\"}\n",
        "the login is readable at the name Claude looks for it under"
    );
    assert_eq!(
        std::fs::metadata(root.join(".credentials.json"))
            .unwrap()
            .ino(),
        std::fs::metadata(fixture.claude_dir().join(".credentials.json"))
            .unwrap()
            .ino(),
        "and it is the account's own file, which is what a hard link is"
    );

    // And the other direction, which is the half that says the entries are the
    // account rather than a copy of it: what a session writes as memory and
    // transcript has to land in the human's own account.
    let transcript = root
        .join("projects")
        .join(entry_named(fixture.worktree()))
        .join("the-session.jsonl");
    std::fs::write(&transcript, "{}\n").unwrap();

    assert!(
        fixture
            .claude_dir()
            .join("projects")
            .join(entry_named(fixture.worktree()))
            .join("the-session.jsonl")
            .is_file(),
        "a transcript a session writes should be on the account"
    );

    // The file half is a copy rather than a link: written into, it is the
    // session's own until the session ends — see
    // [`a_windows_sessions_config_is_a_copy_merged_into_the_account_as_the_session_ends`].
    assert_ne!(
        std::fs::metadata(profile.join(".claude.json"))
            .unwrap()
            .ino(),
        std::fs::metadata(fixture.claude_config()).unwrap().ino(),
        "the account's config file is copied into the profile, not linked"
    );

    std::fs::write(profile.join(".claude.json"), "{\"logged-in\": true}\n").unwrap();

    assert_eq!(
        std::fs::read_to_string(fixture.claude_config()).unwrap(),
        "{}\n",
        "so what is written inside is not on the account while the session runs"
    );
}

/// An OpenCode session on Windows is given a root of Verkstead's own too: a
/// config directory holding only the `opencode.json` Verkstead wrote, carrying
/// the account's provider, and with memory on the account's data directory
/// junctioned in whole. None of the human's skills, plugins or MCP servers.
#[tokio::test]
async fn a_windows_opencode_session_is_given_a_root_of_the_allowlist_and_nothing_else() {
    let fixture = grilling().await;
    let opencode = fixture.opencode_profile().await;

    made(
        &fixture
            .sandbox_on_under(&opencode, Platform::Windows)
            .expect("a grilling Conversation has a worktree to build a sandbox around"),
    );

    let config = fixture.windows_profile().join(".config/opencode");
    let data = fixture.windows_profile().join(".local/share/opencode");

    assert!(
        !std::fs::symlink_metadata(&config).unwrap().is_symlink(),
        "the config directory is Verkstead's own, not a junction to the account"
    );
    assert_eq!(listed(&config), ["opencode.json"]);
    assert_opencode_config_carries_the_provider_alone(
        &std::fs::read_to_string(config.join("opencode.json")).unwrap(),
    );

    assert!(
        std::fs::symlink_metadata(&data).unwrap().is_symlink(),
        "the data directory is the account's, joined whole"
    );

    std::fs::write(data.join("opencode.db"), "written inside\n").unwrap();

    assert_eq!(
        std::fs::read_to_string(
            fixture
                .opencode_home()
                .join(".local/share/opencode/opencode.db")
        )
        .unwrap(),
        "written inside\n",
        "and a store a session writes is on the account"
    );
}

/// With memory off, a Windows OpenCode root's data directory is its own and
/// holds the account's login alone, hard-linked. A login the session replaces
/// by rename is the account's once the session ends.
#[tokio::test]
async fn a_windows_opencode_root_without_memory_links_the_login_alone_and_hands_it_back() {
    let fixture = grilling().await;
    let forgetting = store::Profile {
        memory: false,
        ..fixture.opencode_profile().await
    };
    let credentials = fixture
        .opencode_home()
        .join(".local/share/opencode/auth.json");

    let afterwards = made(
        &fixture
            .sandbox_on_under(&forgetting, Platform::Windows)
            .expect("a grilling Conversation has a worktree to build a sandbox around"),
    );

    let data = fixture.windows_profile().join(".local/share/opencode");

    assert!(
        !std::fs::symlink_metadata(&data).unwrap().is_symlink(),
        "the data directory is the root's own"
    );
    assert_eq!(listed(&data), ["auth.json"]);
    assert_eq!(
        std::fs::metadata(data.join("auth.json")).unwrap().ino(),
        std::fs::metadata(&credentials).unwrap().ino(),
        "the login is the account's own file, which is what a hard link is"
    );

    replaced(&data.join("auth.json"), "{\"refreshed\": \"inside\"}\n");

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        OPENCODE_LOGIN,
        "the replaced login is not the account's while the session runs"
    );

    afterwards.close();

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        "{\"refreshed\": \"inside\"}\n",
        "and is once it ends"
    );
}

/// A Codex session on Windows is given a root of Verkstead's own too: the login
/// hard-linked into it, `sessions/` and `memories/` junctioned in, and a
/// `config.toml` Verkstead wrote carrying the account's provider and none of
/// its MCP servers. None of the human's rules, skills, instructions, history
/// or databases.
#[tokio::test]
async fn a_windows_codex_session_is_given_a_root_of_the_allowlist_and_nothing_else() {
    let fixture = grilling().await;
    let codex = fixture.codex_profile().await;

    made(
        &fixture
            .sandbox_on_under(&codex, Platform::Windows)
            .expect("a grilling Conversation has a worktree to build a sandbox around"),
    );

    let root = fixture.windows_profile().join(".codex");

    assert!(
        !std::fs::symlink_metadata(&root).unwrap().is_symlink(),
        "the root is a directory of Verkstead's own, not a junction to the account"
    );
    assert_eq!(
        listed(&root),
        ["auth.json", "config.toml", "memories", "sessions"],
        "the login, the written configuration and the memory store are the whole \
         of the root"
    );
    assert_eq!(
        std::fs::metadata(root.join("auth.json")).unwrap().ino(),
        std::fs::metadata(fixture.codex_dir().join("auth.json"))
            .unwrap()
            .ino(),
        "the login is the account's own file, which is what a hard link is"
    );
    assert_codex_config_carries_the_provider_alone(&root.join("config.toml"));

    std::fs::write(root.join("sessions/rollout-the-session.jsonl"), "{}\n").unwrap();
    std::fs::write(root.join("memories/MEMORY.md"), "remembered inside\n").unwrap();

    assert!(
        fixture
            .codex_dir()
            .join("sessions/rollout-the-session.jsonl")
            .is_file(),
        "a rollout a session writes is on the account"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.codex_dir().join("memories/MEMORY.md")).unwrap(),
        "remembered inside\n",
        "and so is a memory"
    );
}

/// With memory off, a Windows Codex root's `sessions/` and `memories/` are its
/// own and empty, and nothing of the account's is joined for them.
#[tokio::test]
async fn a_windows_codex_root_without_memory_has_a_store_of_its_own() {
    let fixture = grilling().await;
    let forgetting = store::Profile {
        memory: false,
        ..fixture.codex_profile().await
    };

    made(
        &fixture
            .sandbox_on_under(&forgetting, Platform::Windows)
            .expect("a grilling Conversation has a worktree to build a sandbox around"),
    );

    let root = fixture.windows_profile().join(".codex");

    assert_eq!(
        listed(&root),
        ["auth.json", "config.toml", "memories", "sessions"]
    );

    for store in ["sessions", "memories"] {
        assert!(
            !std::fs::symlink_metadata(root.join(store))
                .unwrap()
                .is_symlink(),
            "the root's {store} is its own directory"
        );
        assert!(listed(&root.join(store)).is_empty(), "and it starts empty");
    }
}

/// A Codex account with no login has none to link into a Windows root, so the
/// login a session makes is handed back as it ends.
#[tokio::test]
async fn a_codex_login_a_windows_session_made_in_an_account_with_none_is_the_accounts_afterwards() {
    let fixture = grilling().await;
    let codex = fixture.codex_profile().await;
    let credentials = fixture.codex_dir().join("auth.json");
    std::fs::remove_file(&credentials).unwrap();

    let afterwards = made(
        &fixture
            .sandbox_on_under(&codex, Platform::Windows)
            .expect("a grilling Conversation has a worktree to build a sandbox around"),
    );
    let inside = fixture.windows_profile().join(".codex/auth.json");

    assert!(!inside.exists(), "there is no login to give the root");
    std::fs::write(&inside, "{\"logged\": \"in\"}\n").unwrap();

    afterwards.close();

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        "{\"logged\": \"in\"}\n",
        "the login the session made is the account's"
    );
}

/// A Grok Build session on Windows is given a root of Verkstead's own too: the
/// login hard-linked into it, `sessions/` and `memory/` junctioned in, and a
/// `config.toml` Verkstead wrote carrying what reaches the model and none of
/// the human's MCP servers. None of the human's skills, plugins, interface
/// settings or logs.
#[tokio::test]
async fn a_windows_grok_session_is_given_a_root_of_the_allowlist_and_nothing_else() {
    let fixture = grilling().await;
    let grok = fixture.grok_profile().await;

    made(
        &fixture
            .sandbox_on_under(&grok, Platform::Windows)
            .expect("a grilling Conversation has a worktree to build a sandbox around"),
    );

    let root = fixture.windows_profile().join(".grok");

    assert!(
        !std::fs::symlink_metadata(&root).unwrap().is_symlink(),
        "the root is a directory of Verkstead's own, not a junction to the account"
    );
    assert_eq!(
        listed(&root),
        ["auth.json", "config.toml", "memory", "sessions"],
        "the login, the written configuration and the memory store are the whole \
         of the root"
    );
    assert_eq!(
        std::fs::metadata(root.join("auth.json")).unwrap().ino(),
        std::fs::metadata(fixture.grok_dir().join("auth.json"))
            .unwrap()
            .ino(),
        "the login is the account's own file, which is what a hard link is"
    );
    assert_grok_config_carries_the_model_alone(&root.join("config.toml"));

    std::fs::create_dir_all(root.join("sessions/%2Fwork/019-the-session")).unwrap();
    std::fs::write(
        root.join("sessions/%2Fwork/019-the-session/updates.jsonl"),
        "{}\n",
    )
    .unwrap();

    assert!(
        fixture
            .grok_dir()
            .join("sessions/%2Fwork/019-the-session/updates.jsonl")
            .is_file(),
        "a log a session writes is on the account"
    );
}

/// A login grok 1.0.13 saves inside a Windows root replaces the hard link, and
/// is the account's once the session ends. A `MEMORY.md` replaced the same way
/// is in a junctioned directory, so it is the account's as it is written.
#[tokio::test]
async fn a_grok_login_a_windows_session_replaced_is_the_accounts_once_the_session_ends() {
    let fixture = grilling().await;
    let grok = fixture.grok_profile().await;

    let afterwards = made(
        &fixture
            .sandbox_on_under(&grok, Platform::Windows)
            .expect("a grilling Conversation has a worktree to build a sandbox around"),
    );
    let root = fixture.windows_profile().join(".grok");

    replaced(&root.join("auth.json"), "{\"refreshed\": \"inside\"}\n");
    replaced(&root.join("memory/MEMORY.md"), "remembered inside\n");

    assert_eq!(
        std::fs::read_to_string(fixture.grok_dir().join("memory/MEMORY.md")).unwrap(),
        "remembered inside\n",
        "the memory is the account's straight away"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.grok_dir().join("auth.json")).unwrap(),
        GROK_LOGIN,
        "while the replaced login is not, until the session ends"
    );

    afterwards.close();

    assert_eq!(
        std::fs::read_to_string(fixture.grok_dir().join("auth.json")).unwrap(),
        "{\"refreshed\": \"inside\"}\n",
        "and then it is"
    );
}

/// Somewhere to write a temporary file, which on this platform is inside the
/// profile rather than shared with everybody on the machine.
#[tokio::test]
async fn what_a_windows_session_throws_away_is_thrown_away_under_its_own_profile() {
    let fixture = grilling().await;

    made(&fixture.sandbox_on(Platform::Windows));

    let temporary = fixture
        .windows_profile()
        .join("AppData")
        .join("Local")
        .join("Temp");

    assert!(
        temporary.is_dir(),
        "TEMP names {}, so it has to be a directory that is really there",
        temporary.display()
    );
}

/// The profile is one Conversation's and is made fresh for each of its
/// sessions: what one left there is not what the next one finds.
///
/// And the account is not, which is the other half of the same claim and the
/// one worth being sure of. The account is joined in by a name, and emptying
/// the profile takes the name rather than what is behind it — anything else
/// would be Verkstead deleting the human's own login.
#[tokio::test]
async fn each_session_gets_the_profile_fresh_and_the_account_untouched() {
    let fixture = grilling().await;

    made(&fixture.sandbox_on(Platform::Windows));

    let profile = fixture.windows_profile();
    std::fs::write(profile.join("what-the-last-session-left"), "state\n").unwrap();
    std::fs::write(profile.join(".claude/left-in-the-root"), "state\n").unwrap();

    made(&fixture.sandbox_on(Platform::Windows));

    assert!(
        !profile.join("what-the-last-session-left").exists()
            && !profile.join(".claude/left-in-the-root").exists(),
        "a session should start in a profile and a root holding nothing of the \
         session before it"
    );

    assert_eq!(
        std::fs::read_to_string(profile.join(".claude/.credentials.json")).unwrap(),
        "{\"the\": \"login\"}\n",
        "and the login should be joined in again"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.claude_dir().join(".credentials.json")).unwrap(),
        "{\"the\": \"login\"}\n",
        "with the account's own file untouched by the emptying"
    );
    assert!(
        fixture
            .claude_dir()
            .join("projects")
            .join(entry_named(fixture.worktree()))
            .is_dir(),
        "and so is each `projects/` entry a junction led to"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.claude_config()).unwrap(),
        "{}\n",
        "the file half included"
    );
    assert!(
        fixture
            .claude_dir()
            .join("skills/the-accounts-own/SKILL.md")
            .exists(),
        "and nothing inside it touched: what a rendering makes it makes in the \
         profile, and there is no boundary on this platform to make anything in \
         the account"
    );
}

/// A Windows session's `.claude.json` is a copy: its Repo and Worktree trusted,
/// the account's MCP servers taken out, and what the session changed merged
/// into the account's own file as it ends — whether it wrote the copy in place
/// or replaced it by rename, as Claude saves one.
///
/// Asked of the description on whichever machine is running this, as everything
/// else about that rendering is.
#[tokio::test]
async fn a_windows_sessions_config_is_a_copy_merged_into_the_account_as_the_session_ends() {
    let fixture = grilling().await;
    std::fs::write(
        fixture.claude_config(),
        "{\"numStartups\": 1, \"theme\": \"dark\", \"mcpServers\": {\"the-humans\": {}}}\n",
    )
    .unwrap();

    let afterwards = made(&fixture.sandbox_on(Platform::Windows));
    let inside = fixture.windows_profile().join(".claude.json");

    let copy: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&inside).unwrap()).unwrap();

    assert_eq!(copy["mcpServers"], serde_json::Value::Null);
    assert_eq!(copy["numStartups"], 1);
    assert_eq!(
        copy["projects"]
            .as_object()
            .unwrap()
            .values()
            .filter(|entry| entry["hasTrustDialogAccepted"] == true)
            .count(),
        2,
        "the Repo and the Worktree are trusted in the copy: {copy}"
    );
    assert_eq!(
        afterwards.copied().collect::<Vec<_>>(),
        [inside.as_path()],
        "and the copy is what the session's ending merges back"
    );
    assert!(
        !afterwards.linked().any(|linked| linked == inside),
        "rather than a link it writes back whole"
    );

    let written = fixture.windows_profile().join(".claude.json.tmp");
    std::fs::write(
        &written,
        "{\"numStartups\": 2, \"theme\": \"dark\", \"projects\": {}}\n",
    )
    .unwrap();
    std::fs::rename(&written, &inside).unwrap();

    std::fs::write(
        fixture.claude_config(),
        "{\"numStartups\": 1, \"theme\": \"light\", \"mcpServers\": {\"the-humans\": {}}}\n",
    )
    .unwrap();

    afterwards.close();

    let merged: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture.claude_config()).unwrap()).unwrap();

    assert_eq!(
        merged["numStartups"], 2,
        "what the session changed is merged in"
    );
    assert_eq!(
        merged["theme"], "light",
        "what the account changed meanwhile is kept"
    );
    assert_eq!(
        merged["mcpServers"],
        serde_json::json!({"the-humans": {}}),
        "and the human's MCP servers survive a copy that never had them"
    );
}

/// A login a Windows session saves by rename is the account's once the session
/// has ended, as Claude saves one: a temporary file renamed over the hard link
/// in the root.
#[tokio::test]
async fn a_login_a_windows_session_saved_by_rename_is_written_back_to_the_account() {
    let fixture = grilling().await;
    let credentials = fixture.claude_dir().join(".credentials.json");

    let afterwards = made(&fixture.sandbox_on(Platform::Windows));
    let inside = fixture.windows_profile().join(".claude/.credentials.json");

    let written = inside.with_extension("json.tmp");
    std::fs::write(&written, "{\"refreshed\": true}\n").unwrap();
    std::fs::rename(&written, &inside).unwrap();

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        "{\"the\": \"login\"}\n",
        "the rename took the root's name off the account's file"
    );

    afterwards.close();

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        "{\"refreshed\": true}\n",
        "so as the session ends the refreshed login is written back over it"
    );
    assert_eq!(
        std::fs::metadata(&inside).unwrap().ino(),
        std::fs::metadata(&credentials).unwrap().ino(),
        "and the root's name is one with the account's file again"
    );
}

/// An account with no login has none to link into a Windows root, so the
/// login a session makes is handed back as the session ends and is the
/// account's from then on.
#[tokio::test]
async fn a_login_a_windows_session_made_in_an_account_with_none_is_the_accounts_afterwards() {
    let fixture = grilling().await;
    let credentials = fixture.claude_dir().join(".credentials.json");
    std::fs::remove_file(&credentials).unwrap();

    let afterwards = made(&fixture.sandbox_on(Platform::Windows));
    let inside = fixture.windows_profile().join(".claude/.credentials.json");

    assert!(!inside.exists(), "there is no login to give the root");
    assert!(
        afterwards.linked().any(|linked| linked == inside),
        "and the name a login would be made at is what the session's ending is \
         asked about"
    );

    std::fs::write(&inside, "{\"logged\": \"in\"}\n").unwrap();

    afterwards.close();

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        "{\"logged\": \"in\"}\n",
        "the login the session made is the account's"
    );
}

/// And what each rendering leaves to be seen to, which is nothing at all on the
/// platform whose links follow their own target — for an account with a login
/// to bind, which is the fixture's. See
/// [`a_login_made_inside_an_account_with_none_is_the_accounts_once_the_session_ends`]
/// for the one that has none.
///
/// A Conversation Terminal holds one of these as a session does — it is a shell
/// in the same profile under the same account. The Mac's is asked of the
/// rendering itself, in the server's own tests, because a Mac sandbox is not
/// one this machine can build.
#[tokio::test]
async fn a_rendering_whose_links_follow_their_target_leaves_nothing_to_close() {
    let fixture = grilling().await;

    assert_eq!(
        made(&fixture.sandbox_on(Platform::Linux)).linked().count(),
        0,
        "a bind is a name that follows whatever happens at the far end of it, so \
         a session that ends leaves nothing to see to"
    );

    let linked: Vec<PathBuf> = made(&fixture.sandbox_on(Platform::Windows))
        .linked()
        .map(Path::to_owned)
        .collect();

    assert!(
        linked.contains(&fixture.windows_profile().join(".claude/.credentials.json")),
        "and the platform that joins a file in by hard link leaves the file it \
         joined in, the login in the root: {linked:?}"
    );
    assert!(
        !linked.contains(&fixture.windows_profile().join(".claude.json")),
        "but not `.claude.json`, which is a copy rather than a link: {linked:?}"
    );

    // And nothing else anywhere. A name outside the profile is a rendering
    // about to clear and link somewhere on the *host* — which on the machine
    // running this is its own `/verkstead`, and is what asking every one of
    // these for the platform it renders for is there to stop. See
    // `sandbox_on_under`.
    assert!(
        linked
            .iter()
            .all(|inside| inside.starts_with(fixture.windows_profile())),
        "everything a Windows rendering joins in by hand is inside the profile \
         it is building: {linked:?}"
    );
}

/// An account on another volume from the Data Directory is a session that is
/// not started.
///
/// A hard link is two names for one file, and one volume is the whole of what
/// it needs. What is refused is the sandbox — which is how every other thing a
/// session cannot be given is refused — rather than found out as the link fails
/// and a session starts logged out with nothing saying why.
#[tokio::test]
async fn an_account_on_another_volume_is_a_session_that_is_not_started() {
    let fixture = grilling().await;

    let elsewhere = store::create_profile(
        &fixture.pool,
        &store::ProfileFacts {
            name: Some("on-the-other-drive".to_owned()),
            // The directory half where the fixture's own is, so that what is
            // being refused is the one path a hard link is asked for rather
            // than the whole account being somewhere odd.
            account: store::Account::Claude {
                claude_dir: fixture.claude_dir(),
                config_file: PathBuf::from(r"Z:\accounts\work\.claude.json"),
            },
            models: vec!["claude-opus-5".to_owned()],
            memory: true,
        },
    )
    .await
    .unwrap()
    .expect("nothing is called that yet");

    assert!(
        fixture
            .sandbox_on_under(&elsewhere, Platform::Windows)
            .is_none(),
        "there is no way to join that account into a profile under the Data \
         Directory, so there is no sandbox to run a session in"
    );

    assert!(
        fixture
            .sandbox_on_under(&fixture.profile, Platform::Windows)
            .is_some(),
        "and an account on the Data Directory's own volume is a session that \
         runs, which is what says the refusal above is about the volume"
    );
}

/// Render `sandbox`, throw the rendering away and keep what its ending is left
/// to see to — which is what makes everything the description says is made.
///
/// The rendering itself is what the tests above it read; these are about what
/// is on the disk by the time there is one — see the open rendering's
/// `realise`, which the seatbelt rendering does the same thing in. What comes
/// back is what a session's relay holds until the process has gone, and the two
/// tests about a session's ending are the only ones that ask it anything.
fn made(sandbox: &Sandbox) -> Closing {
    sandbox
        .command(&["the-agent"])
        .expect("a rendering on a platform with no identity to make")
        .1
}

/// GitHub auth is said rather than found: the token the human configured, in
/// the environment `gh` reads it out of, and no gh files anywhere.
#[tokio::test]
async fn the_configured_token_is_in_the_environment_and_the_hosts_gh_login_is_not_inside() {
    let fixture = grilling().await;
    fixture.configure_github_token("github_token: ghp_theconfiguredone\n");
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            say token "${{GH_TOKEN-unset}}"
            file "$HOME/.config/gh/hosts.yml" gh
            dir "$HOME/.config/gh" gh-dir
            dir {outside} gh-where-the-host-might-keep-it
            "#,
            outside = quoted(&fixture.home_path().join(".xdg-config")),
        ),
    );

    assert_eq!(
        reported["token"], "ghp_theconfiguredone",
        "`gh` inside authenticates as whoever the settings file says"
    );
    assert_eq!(
        reported["gh"], "absent",
        "nothing of the host's gh login comes in: the token is the whole of it"
    );
    assert_eq!(reported["gh-dir"], "absent");
    assert_eq!(
        reported["gh-where-the-host-might-keep-it"], "absent",
        "and not under whatever the host's XDG_CONFIG_HOME called it either"
    );
}

/// The three ways there is no token — no file, an empty one, and one nothing
/// can parse — are one answer: a session that starts, with `gh` inside saying
/// for itself that it is not logged in.
#[tokio::test]
async fn no_token_configured_is_a_session_that_starts_with_no_gh_token() {
    for configured in [None, Some(""), Some("github_token: [oh\n")] {
        let fixture = grilling().await;

        if let Some(yaml) = configured {
            fixture.configure_github_token(yaml);
        }

        let reported = probe(&fixture.sandbox(vec![]), r#"say token "${GH_TOKEN-unset}""#);

        assert_eq!(
            reported["token"], "unset",
            "with {configured:?} in secrets.yaml the variable should not be there at all: \
             an empty GH_TOKEN is a login gh fails on obscurely"
        );
    }
}

#[tokio::test]
async fn the_profiles_pair_is_the_whole_of_what_home_holds() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    // Every path here is `$HOME`'s, which the sandbox sets: what is being asked
    // is what a session finds when it looks where it lives.
    let reported = probe(
        &sandbox,
        r#"
            dir "$HOME/.claude" claude-dir
            file "$HOME/.claude.json" claude-config
            file "$HOME/.ssh/id_ed25519" private-key
            file "$HOME/.bash_history" history
            say home "$(ls -A "$HOME" | sort | tr '\n' ' ')"
        "#,
    );

    assert_eq!(
        reported["claude-dir"], "write",
        "a session writes its own session logs and settings"
    );
    assert_eq!(reported["claude-config"], "write");
    assert_eq!(reported["private-key"], "absent");
    assert_eq!(reported["history"], "absent");
    assert_eq!(
        reported["home"], ".claude .claude.json ",
        "everything else in HOME is simply not there"
    );
}

/// A Codex session's `.codex` is a root of Verkstead's own, holding the
/// account's login, its `sessions/` and `memories/`, and a `config.toml`
/// Verkstead wrote — and nothing else of the account's at all. And it is the
/// whole of what HOME holds: a `~/.claude` inside a Codex session would be an
/// account nothing is running as.
///
/// The human's rules, skills, `AGENTS.md`, history, archive and databases are
/// how *they* work, so none of it is there.
#[tokio::test]
async fn a_codex_session_is_given_a_root_of_the_allowlist_and_nothing_else() {
    let fixture = grilling().await;
    let profile = fixture.codex_profile().await;
    let sandbox = fixture.sandbox_under(&profile, LISTENING, &BuildCache::none(), vec![]);

    let reported = probe(
        &sandbox,
        r#"
            say home "$(ls -A "$HOME" | sort | tr '\n' ' ')"
            say root "$(ls -A "$HOME/.codex" | sort | tr '\n' ' ')"
            file "$HOME/.codex/auth.json" login
            file "$HOME/.codex/sessions/2026/09/01/rollout-the-humans.jsonl" rollout
            file "$HOME/.codex/memories/MEMORY.md" memory
            dir "$HOME/.codex/rules" rules
            dir "$HOME/.codex/skills" skills
            file "$HOME/.codex/AGENTS.md" agents-md
            file "$HOME/.codex/history.jsonl" history
            dir "$HOME/.codex/archived_sessions" archived
            file "$HOME/.codex/state_5.sqlite" state
            dir "$HOME/.claude" claude-dir
            printf '{}\n' > "$HOME/.codex/sessions/rollout-the-session.jsonl"
            printf 'remembered inside\n' > "$HOME/.codex/memories/MEMORY.md"
        "#,
    );

    assert_eq!(reported["home"], ".codex ");
    assert_eq!(
        reported["root"], "auth.json config.toml memories sessions ",
        "the login, the written configuration and the memory store are the whole \
         of the root"
    );

    for write in ["login", "rollout", "memory"] {
        assert_eq!(
            reported[write], "write",
            "the account's {write} is there, and a session can write it"
        );
    }

    for absent in [
        "rules",
        "skills",
        "agents-md",
        "history",
        "archived",
        "state",
        "claude-dir",
    ] {
        assert_eq!(
            reported[absent], "absent",
            "the account's {absent} is none of a session's business"
        );
    }

    assert!(
        fixture
            .codex_dir()
            .join("sessions/rollout-the-session.jsonl")
            .is_file(),
        "a rollout a session writes is on the account, where it is looked for"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.codex_dir().join("memories/MEMORY.md")).unwrap(),
        "remembered inside\n",
        "and so is a memory"
    );

    assert_codex_config_carries_the_provider_alone(
        &fixture.windows_profile().join(".codex/config.toml"),
    );
    assert_eq!(
        std::fs::read_to_string(fixture.codex_dir().join("config.toml")).unwrap(),
        CODEX_ACCOUNT_CONFIG,
        "and the account's own file is as it was"
    );
}

/// An account with no `config.toml` of its own, and a memory store it has
/// never written, still launches: the root is given an empty configuration,
/// and the two directories are made in the account for the joins.
#[tokio::test]
async fn a_codex_account_with_no_config_and_no_store_still_launches() {
    let fixture = grilling().await;
    let profile = fixture.codex_profile().await;

    for gone in ["sessions", "memories"] {
        std::fs::remove_dir_all(fixture.codex_dir().join(gone)).unwrap();
    }
    std::fs::remove_file(fixture.codex_dir().join("config.toml")).unwrap();

    let reported = probe(
        &fixture.sandbox_under(&profile, LISTENING, &BuildCache::none(), vec![]),
        r#"
            say config "$(cat "$HOME/.codex/config.toml")"
            dir "$HOME/.codex/sessions" sessions
            dir "$HOME/.codex/memories" memories
        "#,
    );

    assert_eq!(reported["config"], "", "an empty configuration");
    assert_eq!(reported["sessions"], "write");
    assert_eq!(reported["memories"], "write");
    assert!(
        fixture.codex_dir().join("sessions").is_dir()
            && fixture.codex_dir().join("memories").is_dir(),
        "made in the account, where what is written in them belongs"
    );
    assert!(
        !fixture.codex_dir().join("config.toml").exists(),
        "and the account is not given a configuration file for it"
    );
}

/// With the Profile's memory switched off, a Codex session's `sessions/` and
/// `memories/` are the root's own and start empty: nothing of the account's is
/// joined, and what the session writes stays out of the account.
///
/// The rollout it writes is in the root on the host, under the Conversation's
/// own directory — which is where it is looked for.
#[tokio::test]
async fn a_codex_root_without_memory_has_a_store_of_its_own() {
    let fixture = grilling().await;
    let forgetting = store::Profile {
        memory: false,
        ..fixture.codex_profile().await
    };

    let reported = probe(
        &fixture.sandbox_under(&forgetting, LISTENING, &BuildCache::none(), vec![]),
        r#"
            say root "$(ls -A "$HOME/.codex" | sort | tr '\n' ' ')"
            say sessions "$(ls -A "$HOME/.codex/sessions" | tr '\n' ' ')"
            say memories "$(ls -A "$HOME/.codex/memories" | tr '\n' ' ')"
            file "$HOME/.codex/auth.json" login
            mkdir -p "$HOME/.codex/sessions/2026/09/17"
            printf '{}\n' > "$HOME/.codex/sessions/2026/09/17/rollout-the-session.jsonl"
            printf 'fresh\n' > "$HOME/.codex/memories/MEMORY.md"
        "#,
    );

    assert_eq!(
        reported["root"], "auth.json config.toml memories sessions ",
        "the same root as with memory on"
    );
    assert_eq!(reported["sessions"], "", "but its `sessions/` starts empty");
    assert_eq!(reported["memories"], "", "and so do its `memories/`");
    assert_eq!(
        reported["login"], "write",
        "while the login is joined either way"
    );

    assert!(
        fixture
            .windows_profile()
            .join(".codex/sessions/2026/09/17/rollout-the-session.jsonl")
            .is_file(),
        "the rollout is in the root on the host"
    );
    assert!(
        !fixture.codex_dir().join("sessions/2026/09/17").exists(),
        "and not in the account"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.codex_dir().join("memories/MEMORY.md")).unwrap(),
        "remembered\n",
        "whose memory is untouched"
    );
}

/// A Codex login written inside is the account's as it is written: codex 0.154
/// writes `auth.json` in place, through the bind, so there is nothing to write
/// back as the session ends.
#[tokio::test]
async fn a_codex_login_written_inside_is_the_accounts_as_it_is_written() {
    let fixture = grilling().await;
    let profile = fixture.codex_profile().await;

    let (_, afterwards) = probe_closing(
        &fixture.sandbox_under(&profile, LISTENING, &BuildCache::none(), vec![]),
        r#"printf '{"refreshed": "in place"}\n' > "$HOME/.codex/auth.json""#,
    );

    assert_eq!(
        std::fs::read_to_string(fixture.codex_dir().join("auth.json")).unwrap(),
        "{\"refreshed\": \"in place\"}\n"
    );
    assert_eq!(afterwards.linked().count(), 0);
}

/// A Codex account with no login has no file to bind, so a session that logs in
/// writes one into its own root — and that file is the account's once the
/// session has ended.
#[tokio::test]
async fn a_codex_login_made_inside_an_account_with_none_is_the_accounts_once_the_session_ends() {
    let fixture = grilling().await;
    let profile = fixture.codex_profile().await;
    let credentials = fixture.codex_dir().join("auth.json");
    std::fs::remove_file(&credentials).unwrap();

    let (reported, afterwards) = probe_closing(
        &fixture.sandbox_under(&profile, LISTENING, &BuildCache::none(), vec![]),
        r#"
            file "$HOME/.codex/auth.json" before
            printf '{"logged": "in"}\n' > "$HOME/.codex/auth.json"
        "#,
    );

    assert_eq!(reported["before"], "absent", "there is no login to give it");
    assert!(!credentials.exists());

    afterwards.close();

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        "{\"logged\": \"in\"}\n",
        "and the account's from then on"
    );
}

/// A Conversation Terminal opened while a Codex session runs shares its root
/// rather than emptying it, as a Claude one does. And a root of the other
/// harness beside it in the same Conversation — a Claude session with a
/// terminal under a Codex account — is its own: building one leaves the other
/// as it is.
#[tokio::test]
async fn a_codex_root_something_is_running_in_is_shared_and_a_claude_root_beside_it_kept() {
    let fixture = grilling().await;
    let codex = fixture.codex_profile().await;
    let home = fixture.windows_profile();

    let claude_session = made(&fixture.sandbox(vec![]));
    std::fs::write(home.join(".claude/written-by-claude"), "kept\n").unwrap();

    let codex_sandbox = fixture.sandbox_under(&codex, LISTENING, &BuildCache::none(), vec![]);
    let codex_session = made(&codex_sandbox);

    assert!(
        home.join(".claude/written-by-claude").is_file(),
        "a Codex root built beside a running Claude session leaves its root as it is"
    );

    std::fs::write(home.join(".codex/written-by-codex"), "kept\n").unwrap();
    let terminal = made(&codex_sandbox);

    assert!(
        home.join(".codex/written-by-codex").is_file(),
        "a terminal opened beside a running Codex session shares its root"
    );

    drop(codex_session);
    drop(terminal);
    made(&codex_sandbox);

    assert!(
        !home.join(".codex/written-by-codex").exists(),
        "and once nothing is running in it, the next launch is given it fresh"
    );
    drop(claude_session);
}

/// A Grok Build session's `.grok` is a root of Verkstead's own, holding the
/// account's login, its `sessions/` and `memory/`, and a `config.toml`
/// Verkstead wrote — and nothing else of the account's at all. And it is the
/// whole of what HOME holds.
///
/// The human's skills, plugins, interface settings and logs are how *they*
/// work, so none of it is there.
///
/// **The login is a copy on Linux** rather than a bind, because grok saves one
/// by a rename a bind refuses — so the file inside is the account's as it was
/// when the session started, readable by its owner alone.
#[tokio::test]
async fn a_grok_session_is_given_a_root_of_the_allowlist_and_nothing_else() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = grilling().await;
    let profile = fixture.grok_profile().await;
    let sandbox = fixture.sandbox_under(&profile, LISTENING, &BuildCache::none(), vec![]);

    let reported = probe(
        &sandbox,
        r#"
            say home "$(ls -A "$HOME" | sort | tr '\n' ' ')"
            say root "$(ls -A "$HOME/.grok" | sort | tr '\n' ' ')"
            say login "$(cat "$HOME/.grok/auth.json")"
            file "$HOME/.grok/sessions/%2Fsrc%2Fthe-humans/019-the-humans/updates.jsonl" log
            file "$HOME/.grok/sessions/session_search.sqlite" search
            file "$HOME/.grok/memory/MEMORY.md" memory
            file "$HOME/.grok/memory/the-humans-0123abcd/index.sqlite" index
            dir "$HOME/.grok/skills" skills
            dir "$HOME/.grok/plugins" plugins
            file "$HOME/.grok/pager.toml" pager
            dir "$HOME/.grok/logs" logs
            dir "$HOME/.claude" claude-dir
            say agent "${VERKSTEAD_AGENT-unset}"
            mkdir -p "$HOME/.grok/sessions/%2Fwork/019-the-session"
            printf '{}\n' > "$HOME/.grok/sessions/%2Fwork/019-the-session/updates.jsonl"
            printf 'remembered inside\n' > "$HOME/.grok/memory/MEMORY.md.saving"
            mv "$HOME/.grok/memory/MEMORY.md.saving" "$HOME/.grok/memory/MEMORY.md"
        "#,
    );

    assert_eq!(reported["home"], ".grok ");
    assert_eq!(
        reported["root"], "auth.json config.toml memory sessions ",
        "the login, the written configuration and the memory store are the whole \
         of the root"
    );
    assert_eq!(
        reported["login"],
        GROK_LOGIN.trim_end(),
        "the account's login is there to be read"
    );

    for write in ["log", "search", "memory", "index"] {
        assert_eq!(
            reported[write], "write",
            "the account's {write} is there, and a session can write it"
        );
    }

    for absent in ["skills", "plugins", "pager", "logs", "claude-dir"] {
        assert_eq!(
            reported[absent], "absent",
            "the account's {absent} is none of a session's business"
        );
    }

    assert_eq!(reported["agent"], "grok");

    assert!(
        fixture
            .grok_dir()
            .join("sessions/%2Fwork/019-the-session/updates.jsonl")
            .is_file(),
        "a log a session writes is on the account, where it is looked for"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.grok_dir().join("memory/MEMORY.md")).unwrap(),
        "remembered inside\n",
        "and so is a memory, replaced by a rename the way grok saves one"
    );

    let root = fixture.windows_profile().join(".grok");
    assert_grok_config_carries_the_model_alone(&root.join("config.toml"));
    assert_eq!(
        std::fs::metadata(root.join("auth.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600,
        "the copy of the login is its owner's alone"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.grok_dir().join("config.toml")).unwrap(),
        GROK_ACCOUNT_CONFIG,
        "and the account's own configuration is as it was"
    );
}

/// An account with no `config.toml` of its own, and a memory store it has
/// never written, still launches: the root is given an empty configuration,
/// and the two directories are made in the account for the joins.
#[tokio::test]
async fn a_grok_account_with_no_config_and_no_store_still_launches() {
    let fixture = grilling().await;
    let profile = fixture.grok_profile().await;

    for gone in ["sessions", "memory"] {
        std::fs::remove_dir_all(fixture.grok_dir().join(gone)).unwrap();
    }
    std::fs::remove_file(fixture.grok_dir().join("config.toml")).unwrap();

    let reported = probe(
        &fixture.sandbox_under(&profile, LISTENING, &BuildCache::none(), vec![]),
        r#"
            say config "$(cat "$HOME/.grok/config.toml")"
            dir "$HOME/.grok/sessions" sessions
            dir "$HOME/.grok/memory" memory
        "#,
    );

    assert_eq!(reported["config"], "", "an empty configuration");
    assert_eq!(reported["sessions"], "write");
    assert_eq!(reported["memory"], "write");
    assert!(
        fixture.grok_dir().join("sessions").is_dir() && fixture.grok_dir().join("memory").is_dir(),
        "made in the account, where what is written in them belongs"
    );
    assert!(
        !fixture.grok_dir().join("config.toml").exists(),
        "and the account is not given a configuration file for it"
    );
}

/// With the Profile's memory switched off, a Grok Build session's `sessions/`
/// and `memory/` are the root's own and start empty: nothing of the account's
/// is joined, and what the session writes stays out of the account.
///
/// The log it writes is in the root on the host, under the Conversation's own
/// directory — which is where it is looked for.
#[tokio::test]
async fn a_grok_root_without_memory_has_a_store_of_its_own() {
    let fixture = grilling().await;
    let forgetting = store::Profile {
        memory: false,
        ..fixture.grok_profile().await
    };

    let reported = probe(
        &fixture.sandbox_under(&forgetting, LISTENING, &BuildCache::none(), vec![]),
        r#"
            say root "$(ls -A "$HOME/.grok" | sort | tr '\n' ' ')"
            say sessions "$(ls -A "$HOME/.grok/sessions" | tr '\n' ' ')"
            say memory "$(ls -A "$HOME/.grok/memory" | tr '\n' ' ')"
            file "$HOME/.grok/auth.json" login
            mkdir -p "$HOME/.grok/sessions/%2Fwork/019-the-session"
            printf '{}\n' > "$HOME/.grok/sessions/%2Fwork/019-the-session/updates.jsonl"
            printf 'fresh\n' > "$HOME/.grok/memory/MEMORY.md"
        "#,
    );

    assert_eq!(
        reported["root"], "auth.json config.toml memory sessions ",
        "the same root as with memory on"
    );
    assert_eq!(reported["sessions"], "", "but its `sessions/` starts empty");
    assert_eq!(reported["memory"], "", "and so does its `memory/`");
    assert_eq!(
        reported["login"], "write",
        "while the login is given either way"
    );

    assert!(
        fixture
            .windows_profile()
            .join(".grok/sessions/%2Fwork/019-the-session/updates.jsonl")
            .is_file(),
        "the log is in the root on the host"
    );
    assert!(
        !fixture.grok_dir().join("sessions/%2Fwork").exists(),
        "and not in the account"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.grok_dir().join("memory/MEMORY.md")).unwrap(),
        "remembered\n",
        "whose memory is untouched"
    );
}

/// A login grok saves inside a Linux session — by renaming a file over its
/// copy, which a bind would have refused — is merged into the account as the
/// session ends: the scope it saved goes back, and a scope the human's own
/// `grok` saved meanwhile stays.
#[tokio::test]
async fn a_grok_login_saved_inside_by_rename_is_merged_into_the_account_as_the_session_ends() {
    let fixture = grilling().await;
    let profile = fixture.grok_profile().await;
    let credentials = fixture.grok_dir().join("auth.json");

    let (_, afterwards) = probe_closing(
        &fixture.sandbox_under(&profile, LISTENING, &BuildCache::none(), vec![]),
        r#"
            printf '{"https://auth.x.ai::the-humans": {"key": "refreshed inside"}}\n' \
                > "$HOME/.grok/auth.json.saving"
            mv "$HOME/.grok/auth.json.saving" "$HOME/.grok/auth.json"
        "#,
    );

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        GROK_LOGIN,
        "the account's login is as it was while the session runs"
    );

    // The human's own `grok` signs in to a second scope in the meantime.
    std::fs::write(
        &credentials,
        "{\"https://auth.x.ai::the-humans\": {\"key\": \"the login\"}, \
         \"https://id.example::work\": {\"key\": \"the human's\"}}\n",
    )
    .unwrap();

    afterwards.close();

    let merged: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&credentials).unwrap()).unwrap();

    assert_eq!(
        merged,
        serde_json::json!({
            "https://auth.x.ai::the-humans": { "key": "refreshed inside" },
            "https://id.example::work": { "key": "the human's" },
        }),
        "the session's refresh reaches the account beside the human's own login"
    );
}

/// A Grok Build account with no login has none to copy, so a session that logs
/// in writes one into its own root — and that file is the account's once the
/// session has ended.
#[tokio::test]
async fn a_grok_login_made_inside_an_account_with_none_is_the_accounts_once_the_session_ends() {
    let fixture = grilling().await;
    let profile = fixture.grok_profile().await;
    let credentials = fixture.grok_dir().join("auth.json");
    std::fs::remove_file(&credentials).unwrap();

    let (reported, afterwards) = probe_closing(
        &fixture.sandbox_under(&profile, LISTENING, &BuildCache::none(), vec![]),
        r#"
            file "$HOME/.grok/auth.json" before
            printf '{"logged": "in"}\n' > "$HOME/.grok/auth.json"
        "#,
    );

    assert_eq!(reported["before"], "absent", "there is no login to give it");
    assert!(!credentials.exists());

    afterwards.close();

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        "{\"logged\": \"in\"}\n",
        "and the account's from then on"
    );
}

/// An OpenCode session is given the two directories opencode reads, at the XDG
/// defaults inside a fresh HOME — so nothing has to be said in the environment
/// about where they are — built from an allowlist.
///
/// **The config directory is Verkstead's own**, holding only an `opencode.json`
/// that carries the account's provider, read out of its `opencode.jsonc`,
/// comments and all. None of the human's skills, plugins, MCP servers or
/// instructions.
///
/// **The data directory is the account's, joined whole** with memory on: the
/// login and the database beside its write-ahead-log siblings, so what a
/// session writes there is on the account.
///
/// The cache and the state directories are the sandbox's own, made fresh and
/// thrown away with it. And the store's name is pinned in the environment, so
/// a session writes the one file Verkstead named.
#[tokio::test]
async fn an_opencode_session_is_given_a_root_of_the_allowlist_and_nothing_else() {
    let fixture = grilling().await;
    let profile = fixture.opencode_profile().await;
    let sandbox = fixture.sandbox_under(&profile, LISTENING, &BuildCache::none(), vec![]);

    let reported = probe(
        &sandbox,
        r#"
            say config "$(ls -A "$HOME/.config/opencode" | sort | tr '\n' ' ')"
            say data "$(ls -A "$HOME/.local/share/opencode" | sort | tr '\n' ' ')"
            say login "$(cat "$HOME/.local/share/opencode/auth.json")"
            file "$HOME/.local/share/opencode/opencode.db" db-file
            dir "$HOME/.claude" claude-dir
            say home "$(ls -A "$HOME" | sort | tr '\n' ' ')"
            say cache "$(ls -A "$HOME/.cache" 2>/dev/null | tr '\n' ' ')"
            say db "${OPENCODE_DB-unset}"
            say agent "${VERKSTEAD_AGENT-unset}"
            printf 'written inside\n' > "$HOME/.local/share/opencode/opencode.db"
        "#,
    );

    assert_eq!(
        reported["config"], "opencode.json ",
        "the written configuration is the whole of the config directory"
    );
    assert_opencode_config_carries_the_provider_alone(
        &std::fs::read_to_string(
            fixture
                .windows_profile()
                .join(".config/opencode/opencode.json"),
        )
        .unwrap(),
    );
    assert_eq!(
        reported["data"], "auth.json mcp-auth.json opencode.db opencode.db-shm opencode.db-wal ",
        "the account's data directory, whole"
    );
    assert_eq!(reported["login"], OPENCODE_LOGIN.trim_end());
    assert_eq!(reported["db-file"], "write");
    assert_eq!(reported["claude-dir"], "absent");
    assert_eq!(
        reported["home"], ".config .local ",
        "and nothing else of HOME is there — the cache and the state opencode \
         makes are the sandbox's own"
    );
    assert_eq!(
        reported["cache"], "",
        "the cache directory is not the account's, so nothing of it is bound"
    );
    assert_eq!(
        reported["db"], "opencode.db",
        "the store is named by Verkstead rather than by the release channel"
    );
    assert_eq!(reported["agent"], "opencode");

    let data = fixture.opencode_home().join(".local/share/opencode");
    assert_eq!(
        std::fs::read_to_string(data.join("opencode.db")).unwrap(),
        "written inside\n",
        "a store a session writes is the account's, where it is looked for"
    );
    assert_eq!(
        std::fs::read_to_string(
            fixture
                .opencode_home()
                .join(".config/opencode/opencode.jsonc")
        )
        .unwrap(),
        OPENCODE_ACCOUNT_CONFIG,
        "and the account's own configuration is as it was"
    );
}

/// With the Profile's memory switched off, an OpenCode session's data directory
/// is the root's own: only the account's login is linked into it, and the
/// database starts empty. What the session writes stays out of the account, and
/// is in the root on the host, under the Conversation's own directory — which
/// is where it is looked for. A login written inside is the account's as it is
/// written, opencode writing `auth.json` in place.
#[tokio::test]
async fn an_opencode_root_without_memory_has_a_data_directory_of_its_own() {
    let fixture = grilling().await;
    let forgetting = store::Profile {
        memory: false,
        ..fixture.opencode_profile().await
    };

    let reported = probe(
        &fixture.sandbox_under(&forgetting, LISTENING, &BuildCache::none(), vec![]),
        r#"
            say config "$(ls -A "$HOME/.config/opencode" | sort | tr '\n' ' ')"
            say data "$(ls -A "$HOME/.local/share/opencode" | sort | tr '\n' ' ')"
            file "$HOME/.local/share/opencode/auth.json" login
            printf 'the session'"'"'s own\n' > "$HOME/.local/share/opencode/opencode.db"
            printf '{"refreshed": "in place"}\n' > "$HOME/.local/share/opencode/auth.json"
        "#,
    );

    assert_eq!(
        reported["config"], "opencode.json ",
        "the same config directory as with memory on"
    );
    assert_eq!(
        reported["data"], "auth.json ",
        "and of the account's data directory, the login alone"
    );
    assert_eq!(reported["login"], "write");

    assert_eq!(
        std::fs::read_to_string(
            fixture
                .windows_profile()
                .join(".local/share/opencode/opencode.db")
        )
        .unwrap(),
        "the session's own\n",
        "the store is in the root on the host"
    );

    let data = fixture.opencode_home().join(".local/share/opencode");
    assert_eq!(
        std::fs::read_to_string(data.join("opencode.db")).unwrap(),
        "the human's sessions\n",
        "and the account's is untouched"
    );
    assert_eq!(
        std::fs::read_to_string(data.join("auth.json")).unwrap(),
        "{\"refreshed\": \"in place\"}\n",
        "while the login written inside is the account's"
    );
}

/// An OpenCode account with no login and memory off has no file to bind, so a
/// session that logs in writes one into its own root — and that file is the
/// account's once the session has ended.
#[tokio::test]
async fn an_opencode_login_made_inside_an_account_with_none_is_the_accounts_once_the_session_ends()
{
    let fixture = grilling().await;
    let forgetting = store::Profile {
        memory: false,
        ..fixture.opencode_profile().await
    };
    let credentials = fixture
        .opencode_home()
        .join(".local/share/opencode/auth.json");
    std::fs::remove_file(&credentials).unwrap();

    let (reported, afterwards) = probe_closing(
        &fixture.sandbox_under(&forgetting, LISTENING, &BuildCache::none(), vec![]),
        r#"
            file "$HOME/.local/share/opencode/auth.json" before
            printf '{"logged": "in"}\n' > "$HOME/.local/share/opencode/auth.json"
        "#,
    );

    assert_eq!(reported["before"], "absent", "there is no login to give it");
    assert!(!credentials.exists());

    afterwards.close();

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        "{\"logged\": \"in\"}\n",
        "and the account's from then on"
    );
}

/// And an OpenCode session's shell tool holds a command for a day rather than
/// for two minutes, which is what a blocking ask under this backend stands on.
///
/// The Guide tells an OpenCode session to pass a timeout of its own, and this is
/// what a session that did not gets: opencode's own default is two minutes, and
/// `verkstead ask` blocks until a human on a phone answers. Without it a
/// drifted instruction is an ask killed minutes into a wait measured in hours,
/// with the human's answer landing where nothing is waiting for it.
///
/// This backend's alone, as the store's name is: it is opencode's spelling for
/// opencode's tool, and a sandbox running any other backend has no such tool to
/// tell anything.
#[tokio::test]
async fn an_opencode_session_holds_a_shell_command_far_longer_than_opencodes_own_default() {
    let fixture = grilling().await;
    let opencode = fixture.opencode_profile().await;

    let reported = probe(
        &fixture.sandbox_under(&opencode, LISTENING, &BuildCache::none(), vec![]),
        r#"
            say timeout "${OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS-unset}"
        "#,
    );

    assert_eq!(
        reported["timeout"], "86400000",
        "a day in milliseconds, which is the order the wait itself is: a Set \
         asked in the evening is answered the next morning"
    );

    let reported = probe(
        &fixture.sandbox(vec![]),
        r#"
            say timeout "${OPENCODE_EXPERIMENTAL_BASH_DEFAULT_TIMEOUT_MS-unset}"
        "#,
    );

    assert_eq!(
        reported["timeout"], "unset",
        "and nothing is said to a backend with no such tool to say it to"
    );
}

/// A Claude session is told not to update itself, and no other backend is told
/// anything of the sort.
///
/// Claude's native binary keeps its versions under
/// `~/.local/share/claude/versions/` and writes a new one there when it finds
/// one — a directory a session reaches read-only, and the human's own besides:
/// what version they run is theirs to say, and a session that moved it would
/// move it for every session after this one. So the updater is off in every
/// Claude session, wherever `claude` was found.
///
/// Codex is the other half of the claim rather than a second case: the variable
/// is one backend's own spelling, and a session running any other has nothing
/// to read it.
#[tokio::test]
async fn a_claude_session_is_told_not_to_update_the_install_it_is_running() {
    let fixture = grilling().await;
    let codex = fixture.codex_profile().await;

    let reported = probe(
        &fixture.sandbox(vec![]),
        r#"say updater "${DISABLE_AUTOUPDATER-unset}""#,
    );
    assert_eq!(
        reported["updater"], "1",
        "a session never writes into the human's install",
    );

    let reported = probe(
        &fixture.sandbox_under(&codex, LISTENING, &BuildCache::none(), vec![]),
        r#"say updater "${DISABLE_AUTOUPDATER-unset}""#,
    );
    assert_eq!(
        reported["updater"], "unset",
        "and a backend with no such updater is told nothing about one",
    );
}

/// Which backend a session is running is in its environment, and it is there for
/// the Guide: `verkstead guide` inside a sandbox prints the asking instructions
/// for the backend reading them, and nothing else inside says which that is.
#[tokio::test]
async fn a_session_is_told_which_backend_it_is_running() {
    let fixture = grilling().await;
    let codex = fixture.codex_profile().await;

    let reported = probe(
        &fixture.sandbox(vec![]),
        r#"say agent "${VERKSTEAD_AGENT-unset}""#,
    );
    assert_eq!(reported["agent"], "claude");

    let reported = probe(
        &fixture.sandbox_under(&codex, LISTENING, &BuildCache::none(), vec![]),
        r#"say agent "${VERKSTEAD_AGENT-unset}""#,
    );
    assert_eq!(
        reported["agent"], "codex",
        "off the account's own shape, so nothing has to be plumbed through to \
         say which agent is being launched"
    );
}

#[tokio::test]
async fn the_skills_inside_are_the_bundled_ones_and_only_those() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            file "/verkstead/skills/grilling/SKILL.md" grilling
            say installed "$(ls /verkstead/skills | sort | tr '\n' ' ')"
            say verkstead "$(ls /verkstead | sort | tr '\n' ' ')"
            file "$HOME/.claude/CLAUDE.md" claude-md
            dir {tobico} tobico-skills
            if {grep} -q 'verkstead ask' "/verkstead/skills/grilling/SKILL.md"; then
                say ask-instruction inside
            else
                say ask-instruction missing
            fi
            "#,
            tobico = quoted(&fixture.home_path().join("src/tobico-skills")),
            grep = quoted(&on_the_host("grep")),
        ),
    );

    assert_eq!(
        reported["grilling"], "read",
        "the bundled grilling skill is installed, and is not a session's to rewrite"
    );
    assert_eq!(
        reported["installed"],
        installed_on_the_host(&fixture.skills),
        "and the whole of what this binary ships is there, at a path no backend owns"
    );
    assert_eq!(
        reported["verkstead"], "attachments bin skills ",
        "in a directory the binds made, holding what the server put there and nothing \
         else — the attachments directory among them, which every session has whether \
         or not anything is in it"
    );
    assert_eq!(
        reported["tobico-skills"], "absent",
        "the host's own checkout of the skills is no longer bound in"
    );
    assert_eq!(
        reported["claude-md"], "absent",
        "there is no global CLAUDE.md in here to say how to reach the human"
    );
    assert_eq!(
        reported["ask-instruction"], "inside",
        "so the bundled skill has to carry the instruction itself"
    );
}

/// The skill names as they sit on the host, in the shape the probe reports them
/// from inside — so the assertion above is "the installed set, whatever it is
/// this release" rather than a list to be edited every time a skill lands.
fn installed_on_the_host(skills: &Skills) -> String {
    let mut names: Vec<String> = std::fs::read_dir(skills.path())
        .expect("the skills are installed")
        .map(|entry| entry.expect("a readable entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect();

    names.sort();

    names.iter().map(|name| format!("{name} ")).collect()
}

/// A Claude session's `.claude` is a root of Verkstead's own, holding the
/// account's login and this Repo's and this Worktree's `projects/` entries —
/// and nothing else of the account's at all.
///
/// A Profile is an account to run as rather than a second opinion about how to
/// work: the human's plugins, hooks, global `CLAUDE.md` and history are how
/// *they* work, and another repository's transcripts are no session's business.
/// So none of it is covered over or refused. It is simply never put there.
#[tokio::test]
async fn a_claude_session_is_given_a_root_of_the_allowlist_and_nothing_else() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        r#"
        say root "$(ls -A "$HOME/.claude" | sort | tr '\n' ' ')"
        say projects "$(ls -A "$HOME/.claude/projects" | sort | tr '\n' ' ')"
        dir "$HOME/.claude/plugins" plugins
        file "$HOME/.claude/CLAUDE.md" claude-md
        file "$HOME/.claude/history.jsonl" history
        dir "$HOME/.claude/projects/-home-you-src-something-else" another-repository
        dir "$HOME/.claude/skills" skills
        file "$HOME/.claude/skills/the-accounts-own/SKILL.md" the-accounts-own
        file "$HOME/.claude/.credentials.json" credentials
        "#,
    );

    assert_eq!(
        reported["root"], ".credentials.json projects settings.json ",
        "the login, the projects directory and the settings Verkstead wrote are \
         the whole of the root"
    );

    let mut entries = [entry_named(&fixture.repo), entry_named(fixture.worktree())];
    entries.sort();

    assert_eq!(
        reported["projects"],
        format!("{} {} ", entries[0], entries[1]),
        "and under it, this Repo's entry and this Worktree's"
    );

    for absent in [
        "plugins",
        "claude-md",
        "history",
        "another-repository",
        "skills",
        "the-accounts-own",
    ] {
        assert_eq!(
            reported[absent], "absent",
            "the account's {absent} is none of a session's business"
        );
    }

    assert_eq!(
        reported["credentials"], "write",
        "while the login is there, and a session can refresh it"
    );
}

/// A Claude session's root holds a `settings.json` of Verkstead's own, and an
/// account with none of its own still gets one — holding the key that stops a
/// session parking for ever at the bypass-permissions consent, with nobody at
/// its terminal to answer it.
#[tokio::test]
async fn a_fresh_account_is_given_settings_that_skip_the_bypass_consent() {
    let fixture = grilling().await;
    let settings = fixture.claude_dir().join("settings.json");
    std::fs::remove_file(&settings).unwrap();

    let (reported, closing) = probe_closing(
        &fixture.sandbox(vec![]),
        r#"
        file "$HOME/.claude/settings.json" settings
        say written "$(tr -d ' \n' < "$HOME/.claude/settings.json")"
        "#,
    );
    closing.close();

    assert_eq!(reported["settings"], "write");
    assert_eq!(
        reported["written"], r#"{"skipDangerousModePermissionPrompt":true}"#,
        "the bypass key, and nothing else where the account had nothing to carry"
    );
    assert!(
        !settings.exists(),
        "and the account is not given a settings file for it"
    );
}

/// Of the account's own settings, what an API-key login needs comes over and
/// nothing else does: the human's hooks are how *they* work. And what was
/// written into the root is Verkstead's, so the account's file is as it was
/// once the session has ended — even where the session changed its own.
#[tokio::test]
async fn an_accounts_key_helper_and_environment_come_over_and_its_hooks_do_not() {
    let fixture = grilling().await;
    let settings = fixture.claude_dir().join("settings.json");
    let own = concat!(
        "{\n",
        "  \"apiKeyHelper\": \"/usr/local/bin/print-key\",\n",
        "  \"env\": {\"ANTHROPIC_BASE_URL\": \"https://proxy.example\"},\n",
        "  \"hooks\": {\"Stop\": []}\n",
        "}\n",
    );
    std::fs::write(&settings, own).unwrap();

    let (reported, closing) = probe_closing(
        &fixture.sandbox(vec![]),
        r#"
        say written "$(tr -d ' \n' < "$HOME/.claude/settings.json")"
        printf '{"hooks": {"Stop": ["changed inside"]}}\n' > "$HOME/.claude/settings.json"
        "#,
    );
    closing.close();

    assert_eq!(
        reported["written"],
        concat!(
            r#"{"apiKeyHelper":"/usr/local/bin/print-key","#,
            r#""env":{"ANTHROPIC_BASE_URL":"https://proxy.example"},"#,
            r#""skipDangerousModePermissionPrompt":true}"#,
        ),
        "the key helper and the environment beside the bypass key, and no hooks"
    );
    assert_eq!(
        std::fs::read_to_string(&settings).unwrap(),
        own,
        "and the account's own settings are byte for byte what they were"
    );
}

/// What a session writes under the Repo's entry and the Worktree's lands in the
/// account, which is what makes a memory outlive the session and a transcript
/// something Verkstead can follow — and an entry the account did not have yet
/// is made there first.
///
/// The transcript is found by walking one level of the account's `projects/`
/// for `<session-id>.jsonl`, so the file being at that path in the account is
/// the file being found.
#[tokio::test]
async fn memory_and_a_transcript_written_inside_land_in_the_account() {
    let fixture = grilling().await;

    let projects = fixture.claude_dir().join("projects");
    let (repo, worktree) = (
        projects.join(entry_named(&fixture.repo)),
        projects.join(entry_named(fixture.worktree())),
    );

    assert!(
        !repo.exists() && !worktree.exists(),
        "nobody has run Claude in this Repo yet, so the account has neither entry"
    );

    let sandbox = fixture.sandbox(vec![]);

    assert!(
        repo.is_dir() && worktree.is_dir(),
        "so both are made in the account before a session is given them"
    );

    probe(
        &sandbox,
        &format!(
            r#"
            mkdir -p "$HOME/.claude/projects/{repo}/memory"
            printf 'remembered\n' > "$HOME/.claude/projects/{repo}/memory/MEMORY.md"
            printf '{{"turn": 1}}\n' > "$HOME/.claude/projects/{worktree}/the-session.jsonl"
            "#,
            repo = entry_named(&fixture.repo),
            worktree = entry_named(fixture.worktree()),
        ),
    );

    assert_eq!(
        std::fs::read_to_string(repo.join("memory/MEMORY.md")).unwrap(),
        "remembered\n",
        "the Repo's memory is the account's"
    );
    assert_eq!(
        std::fs::read_to_string(worktree.join("the-session.jsonl")).unwrap(),
        "{\"turn\": 1}\n",
        "and so is the session's transcript, one level under `projects/` where it \
         is looked for"
    );
}

/// With the Profile's memory switched off, a Claude session's root holds a
/// `projects/` of its own and nothing in it: none of the account's entries is
/// joined, and none is made in the account for a join that is not there.
///
/// What the session writes there is the root's, on the host, under the
/// Conversation's own directory — which is where its transcript is looked for.
#[tokio::test]
async fn a_root_without_memory_has_an_empty_projects_of_its_own() {
    let fixture = grilling().await;
    let forgetting = store::Profile {
        memory: false,
        ..fixture.profile.clone()
    };

    let account = fixture.claude_dir().join("projects");
    let (repo, worktree) = (
        account.join(entry_named(&fixture.repo)),
        account.join(entry_named(fixture.worktree())),
    );

    let sandbox = fixture.sandbox_under(&forgetting, LISTENING, &BuildCache::none(), vec![]);

    assert!(
        !repo.exists() && !worktree.exists(),
        "nothing is made in the account's `projects/` for a root that joins none of it"
    );

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            say root "$(ls -A "$HOME/.claude" | sort | tr '\n' ' ')"
            say projects "$(ls -A "$HOME/.claude/projects" | tr '\n' ' ')"
            dir "$HOME/.claude/projects/-home-you-src-something-else" another-repository
            file "$HOME/.claude/.credentials.json" credentials
            mkdir -p "$HOME/.claude/projects/{worktree}"
            printf '{{"turn": 1}}\n' > "$HOME/.claude/projects/{worktree}/the-session.jsonl"
            "#,
            worktree = entry_named(fixture.worktree()),
        ),
    );

    assert_eq!(
        reported["root"], ".credentials.json projects settings.json ",
        "the same root as with memory on"
    );
    assert_eq!(reported["projects"], "", "but its `projects/` starts empty");
    assert_eq!(
        reported["another-repository"], "absent",
        "and none of the account's transcripts are in it"
    );
    assert_eq!(
        reported["credentials"], "write",
        "while the login is joined either way"
    );

    let home = fixture
        .state
        .path()
        .join("homes")
        .join(fixture.conversation.id.to_string());

    assert_eq!(
        std::fs::read_to_string(
            home.join(".claude/projects")
                .join(entry_named(fixture.worktree()))
                .join("the-session.jsonl")
        )
        .unwrap(),
        "{\"turn\": 1}\n",
        "the transcript is in the root on the host, one level under `projects/` \
         where it is looked for"
    );
    assert!(!worktree.exists(), "and not in the account");
}

/// A login refreshed inside is the account's login, as it is written.
///
/// Linux binds the file over its name in the root, and Claude saves it by
/// writing a temporary file and renaming it — a rename a bind refuses, which is
/// when Claude writes the file where it is instead. Both halves are asked here:
/// the rename is refused, and the write in place lands.
#[tokio::test]
async fn a_login_refreshed_inside_is_the_accounts_as_it_is_written() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let (reported, afterwards) = probe_closing(
        &sandbox,
        r#"
        printf '{"refreshed": "renamed"}\n' > "$HOME/.claude/.credentials.json.tmp"
        if mv -f "$HOME/.claude/.credentials.json.tmp" "$HOME/.claude/.credentials.json" 2>/dev/null; then
            say renamed yes
        else
            say renamed refused
        fi
        printf '{"refreshed": "in place"}\n' > "$HOME/.claude/.credentials.json"
        "#,
    );

    assert_eq!(
        reported["renamed"], "refused",
        "a rename onto a bound file is refused, which is what sends Claude to \
         writing in place"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.claude_dir().join(".credentials.json")).unwrap(),
        "{\"refreshed\": \"in place\"}\n",
        "and what it writes in place is the account's login"
    );
    assert_eq!(
        afterwards.linked().count(),
        0,
        "so there is nothing to write back as the session ends"
    );
}

/// An account with no login has no file to bind, so a session that logs in
/// writes one into its own root — and that file is the account's once the
/// session has ended.
#[tokio::test]
async fn a_login_made_inside_an_account_with_none_is_the_accounts_once_the_session_ends() {
    let fixture = grilling().await;
    let credentials = fixture.claude_dir().join(".credentials.json");
    std::fs::remove_file(&credentials).unwrap();

    let sandbox = fixture.sandbox(vec![]);

    let (reported, afterwards) = probe_closing(
        &sandbox,
        r#"
        file "$HOME/.claude/.credentials.json" before
        printf '{"logged": "in"}\n' > "$HOME/.claude/.credentials.json.tmp"
        mv -f "$HOME/.claude/.credentials.json.tmp" "$HOME/.claude/.credentials.json"
        "#,
    );

    assert_eq!(reported["before"], "absent", "there is no login to give it");
    assert!(
        !credentials.exists(),
        "and what it wrote is in its root until the session has ended"
    );

    afterwards.close();

    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        "{\"logged\": \"in\"}\n",
        "and the account's from then on"
    );
}

/// The `.claude.json` a Claude session reads is a copy of the account's, with
/// the Repo and the Worktree trusted in it — so the session is past the trust
/// dialog with nobody at its terminal — and with none of the human's own MCP
/// servers in it, at the top level or under an entry.
#[tokio::test]
async fn a_claude_sessions_config_trusts_the_repo_and_the_worktree_and_holds_no_mcp_servers() {
    let fixture = grilling().await;
    std::fs::write(
        fixture.claude_config(),
        concat!(
            r#"{"numStartups": 7, "mcpServers": {"the-humans": {}}, "projects": "#,
            r#"{"/home/you/src/something-else": {"mcpServers": {"its-own": {}}, "allowedTools": []}}}"#,
            "\n",
        ),
    )
    .unwrap();

    let reported = probe(
        &fixture.sandbox(vec![]),
        r#"say config "$(tr -d '\n' < "$HOME/.claude.json")""#,
    );
    let config: serde_json::Value = serde_json::from_str(&reported["config"]).unwrap();

    for trusted in [fixture.repo.as_path(), fixture.worktree()] {
        assert_eq!(
            config["projects"][trusted.to_string_lossy().as_ref()]["hasTrustDialogAccepted"],
            true,
            "{} reads as trusted inside: {config}",
            trusted.display()
        );
    }

    assert_eq!(
        config["numStartups"], 7,
        "the rest of the account's is there"
    );
    assert_eq!(
        config["mcpServers"],
        serde_json::Value::Null,
        "and not the human's MCP servers"
    );
    assert_eq!(
        config["projects"]["/home/you/src/something-else"],
        serde_json::json!({"allowedTools": []}),
        "nor an entry's own"
    );
}

/// What a Claude session changes in its `.claude.json` reaches the account as
/// the session ends, merged into the account's file as it is by then: a key the
/// account changed meanwhile is kept, and the human's MCP servers survive a copy
/// that never had them.
///
/// Written the way Claude writes it: a rename, which a bind over a file refuses,
/// and then in place.
#[tokio::test]
async fn what_a_session_changed_in_its_config_is_merged_into_the_accounts_as_it_ends() {
    let fixture = grilling().await;
    std::fs::write(
        fixture.claude_config(),
        r#"{"numStartups": 1, "theme": "dark", "tipsHistory": {"a": 1}, "mcpServers": {"the-humans": {}}}"#,
    )
    .unwrap();

    let (reported, afterwards) = probe_closing(
        &fixture.sandbox(vec![]),
        r#"
        printf '{"numStartups": 2, "theme": "dark"}\n' > "$HOME/.claude.json.tmp"
        if mv -f "$HOME/.claude.json.tmp" "$HOME/.claude.json" 2>/dev/null; then
            say renamed yes
        else
            say renamed refused
        fi
        printf '{"numStartups": 2, "theme": "dark"}\n' > "$HOME/.claude.json"
        "#,
    );

    assert_eq!(
        reported["renamed"], "refused",
        "a rename onto the bound copy is refused, which sends Claude to writing \
         in place"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.claude_config()).unwrap(),
        r#"{"numStartups": 1, "theme": "dark", "tipsHistory": {"a": 1}, "mcpServers": {"the-humans": {}}}"#,
        "and what it writes is its copy, not the account's file"
    );

    std::fs::write(
        fixture.claude_config(),
        r#"{"numStartups": 1, "theme": "light", "tipsHistory": {"a": 1}, "mcpServers": {"the-humans": {}}}"#,
    )
    .unwrap();

    afterwards.close();

    let merged: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fixture.claude_config()).unwrap()).unwrap();

    assert_eq!(
        merged,
        serde_json::json!({
            "numStartups": 2,
            "theme": "light",
            "mcpServers": {"the-humans": {}},
        }),
        "the session's change and its removal of `tipsHistory` are merged in, the \
         account's own change of theme is kept, the MCP servers survive, and the \
         trust seeded into the copy stays there"
    );
}

/// A session that changed nothing in its `.claude.json` leaves the account's
/// byte for byte as it was, and a login still one with the account's is left
/// alone.
#[tokio::test]
async fn a_session_that_changed_nothing_leaves_the_accounts_config_as_it_was() {
    let fixture = grilling().await;
    let own = "{\"numStartups\":1,   \"mcpServers\": {\"the-humans\": {}}}\n";
    std::fs::write(fixture.claude_config(), own).unwrap();

    let credentials = fixture.claude_dir().join(".credentials.json");
    let modified = std::fs::metadata(&credentials).unwrap().modified().unwrap();

    let (reported, afterwards) = probe_closing(
        &fixture.sandbox(vec![]),
        r#"file "$HOME/.claude.json" config"#,
    );

    assert_eq!(reported["config"], "write");

    afterwards.close();

    assert_eq!(
        std::fs::read_to_string(fixture.claude_config()).unwrap(),
        own,
        "nothing differs, so nothing is written"
    );
    assert_eq!(
        std::fs::read_to_string(&credentials).unwrap(),
        "{\"the\": \"login\"}\n"
    );
    assert_eq!(
        std::fs::metadata(&credentials).unwrap().modified().unwrap(),
        modified,
        "and the login is not written at all"
    );
}

/// The root is a directory under the Data Directory, the Conversation's own,
/// and every session is given it fresh: what the last one left is gone, and
/// the account behind it is untouched.
///
/// And nothing a session writes under HOME lands anywhere on the host but
/// through a bind: a file beside `.claude` is in the namespace and gone with
/// it, where one inside `.claude` is in the root.
#[tokio::test]
async fn the_root_is_built_under_the_data_directory_fresh_for_each_session() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let home = fixture
        .state
        .path()
        .join("homes")
        .join(fixture.conversation.id.to_string());

    probe(
        &sandbox,
        r#"
        printf 'left\n' > "$HOME/.claude/left-in-the-root"
        printf 'left\n' > "$HOME/left-beside-it"
        "#,
    );

    assert!(
        home.join(".claude/projects").is_dir(),
        "the root is built in the Conversation's own directory under the Data \
         Directory"
    );
    assert!(
        home.join(".claude/left-in-the-root").is_file(),
        "and what a session writes in `.claude` is written there"
    );
    assert!(
        !fixture.home_path().join("left-beside-it").exists()
            && !home.join("left-beside-it").exists(),
        "while what it writes beside `.claude` lands nowhere on the host"
    );

    made(&sandbox);

    assert!(
        !home.join(".claude/left-in-the-root").exists(),
        "a session is given the root fresh, with nothing of the one before it"
    );
    assert!(
        home.join(".claude").is_dir(),
        "and made again, for the binds a session starts with to be made into"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.claude_dir().join(".credentials.json")).unwrap(),
        "{\"the\": \"login\"}\n",
        "and the account behind it is untouched by the emptying"
    );
    assert!(
        fixture
            .claude_dir()
            .join("plugins/the-accounts-own")
            .is_dir()
    );
}

/// A launch into a Conversation that already has something running in its root
/// — a Conversation Terminal opened beside a session — shares that root rather
/// than empty it: emptying it would unmount what is joined into the running
/// one. It is built afresh once everything running in it has ended.
#[tokio::test]
async fn a_root_something_is_running_in_is_shared_rather_than_built_again() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let home = fixture
        .state
        .path()
        .join("homes")
        .join(fixture.conversation.id.to_string());

    let session = made(&sandbox);

    std::fs::write(home.join(".claude/written-by-the-session"), "kept\n").unwrap();
    std::fs::write(
        home.join(".claude.json"),
        "{\"written\": \"by the session\"}\n",
    )
    .unwrap();

    let terminal = made(&sandbox);

    assert!(
        home.join(".claude/written-by-the-session").is_file(),
        "a terminal opened beside a running session leaves its root as it is"
    );
    assert_eq!(
        std::fs::read_to_string(home.join(".claude.json")).unwrap(),
        "{\"written\": \"by the session\"}\n",
        "and its copy of `.claude.json` too"
    );
    assert_eq!(
        terminal.copied().collect::<Vec<_>>(),
        [home.join(".claude.json")],
        "which the terminal merges back as well"
    );

    drop(session);
    let second = made(&sandbox);

    assert!(
        home.join(".claude/written-by-the-session").is_file(),
        "the terminal is still running in it"
    );

    drop(terminal);
    drop(second);
    made(&sandbox);

    assert!(
        !home.join(".claude/written-by-the-session").exists(),
        "and once nothing is, the next launch is given it fresh"
    );
}

/// `path` replaced by a file of `contents`, the way grok 1.0.13 saves one:
/// written beside it and renamed over the top.
fn replaced(path: &Path, contents: &str) {
    let beside = path.with_extension("saving");
    std::fs::write(&beside, contents).unwrap();
    std::fs::rename(&beside, path).unwrap();
}

/// That the `config.toml` a Grok Build root was written at `written` carries
/// the fixture account's model and how it signs in, and none of its MCP
/// servers, memory setting or interface.
fn assert_grok_config_carries_the_model_alone(written: &Path) {
    let written = std::fs::read_to_string(written).unwrap();

    for carried in [
        "[model.the-proxy]",
        "base_url = \"https://proxy.example/v1\"",
        "auth_provider_command = \"/usr/local/bin/sign-in\"",
    ] {
        assert!(
            written.contains(carried),
            "what reaches the model is carried over: {written}"
        );
    }

    for left in ["mcp_servers", "memory", "enabled", "screen_mode"] {
        assert!(
            !written.contains(left),
            "and nothing else of the account's is: {written}"
        );
    }
}

/// The names in a directory on the host, sorted.
fn listed(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();

    names
}

/// That the `opencode.json` an OpenCode root was written, as `written`, carries
/// the fixture account's provider, and none of its MCP servers, plugins or
/// instructions.
fn assert_opencode_config_carries_the_provider_alone(written: &str) {
    let written: serde_json::Value = serde_json::from_str(written).unwrap();

    assert_eq!(
        written,
        serde_json::json!({
            "provider": {
                "proxy": {
                    "npm": "@ai-sdk/openai-compatible",
                    "options": { "baseURL": "https://proxy.example/v1" },
                },
            },
        }),
        "the account's provider is carried over, and nothing else of its configuration"
    );
}

/// That the `config.toml` a Codex root was written at `written` carries the
/// fixture account's provider, and none of its MCP servers, notifier or
/// features.
fn assert_codex_config_carries_the_provider_alone(written: &Path) {
    let written = std::fs::read_to_string(written).unwrap();

    for carried in [
        "model_provider = \"proxy\"",
        "[model_providers.proxy]",
        "base_url = \"https://proxy.example/v1\"",
    ] {
        assert!(
            written.contains(carried),
            "the account's provider is carried over: {written}"
        );
    }

    for left in ["mcp_servers", "notify", "features", "memories"] {
        assert!(
            !written.contains(left),
            "and nothing else of the account's is: {written}"
        );
    }
}

/// The name Claude Code gives a path's `projects/` entry, for the paths these
/// tests use — none of them long enough to be cut and hashed, which is the
/// server's own unit tests' to ask.
fn entry_named(path: &Path) -> String {
    path.canonicalize()
        .unwrap()
        .to_string_lossy()
        .chars()
        .map(|kept| {
            if kept.is_ascii_alphanumeric() {
                kept
            } else {
                '-'
            }
        })
        .collect()
}

/// `verkstead` inside is the executable serving the session, and it is what a
/// bare `verkstead` finds.
///
/// The two halves of an ask are the CLI a session runs and the server it puts a
/// Set to, and they have to be one build: a machine's install is a separate one,
/// and the two have already disagreed about what a `proposal` may carry. So the
/// server hands over its own image, in a directory holding nothing else, ahead
/// of every path the host could have installed a `verkstead` on.
///
/// Asked of a shell inside rather than of the flags, like everything else here:
/// what settles which binary a session asks with is a session looking one up and
/// running it.
#[tokio::test]
async fn the_verkstead_a_session_asks_with_is_the_one_serving_it() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        r#"
        found=$(command -v verkstead)
        say found "$found"
        say version "$(verkstead)"
        say beside "$(ls "$(dirname "$found")")"
        say first "${PATH%%:*}"
        file "$found" binary
        "#,
    );

    assert_eq!(
        reported["found"], "/verkstead/bin/verkstead",
        "a bare `verkstead` is the one the server bound in"
    );
    assert_eq!(
        reported["version"], "verkstead 0.0.0-the-servers-own",
        "and running it runs the server's own build, not whatever the machine has"
    );
    assert_eq!(
        reported["beside"], "verkstead",
        "the directory holds the one executable the server put there and nothing else"
    );
    assert_eq!(
        reported["first"], "/verkstead/bin",
        "which is looked in before every path an install could have landed on"
    );
    assert_eq!(
        reported["binary"], "read",
        "and it is no more a session's to rewrite mid-run than the skills are"
    );
}

/// And `/usr/local/bin` is on it, which is where the Debian family's `npm
/// install -g` lands a binary.
///
/// Asked of a shell inside rather than of the constant it was built from, like
/// everything else here: what settles whether a session would find an agent
/// installed from npm is a session reading the `PATH` it was really handed.
/// That is what the onboarding probes resolve on too — a row that ticked
/// against a list a session did not have would be a row promising a session
/// that could not start. See `verkstead_server::onboarding`.
#[tokio::test]
async fn a_session_looks_for_a_program_where_an_npm_install_puts_one() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(&sandbox, r#"say path "$PATH""#);

    assert!(
        reported["path"]
            .split(':')
            .any(|entry| entry == "/usr/local/bin"),
        "a session's PATH is {:?}, and an agent installed from npm on this \
         family of distributions is nowhere on it",
        reported["path"],
    );
}

/// And where that image was packed with the libraries it runs over, a session
/// still finds one `verkstead` and nothing beside it: the launcher that points
/// the loader at them and execs the image behind it.
///
/// **The AppImage is the whole of why there is a launcher.** It carries GTK and
/// everything under it because the machine it lands on may have none of them,
/// and `AppRun` points the loader there before it execs the binary — but a
/// session gets no `AppRun` and no such variable, so the file alone is a binary
/// that cannot load on exactly the machine the artifact was made for.
///
/// So the libraries go in with it, and the loader is pointed at them for the one
/// binary that needs them rather than for the session: what is in that directory
/// is `libz`, `libexpat` and forty more besides GTK, and a session whose `git`
/// and `rustc` loaded those instead of the machine's would be a session with a
/// quietly re-pointed toolchain. This is that, asked of a shell inside: what the
/// image saw, and what everything else in the sandbox did not.
#[tokio::test]
async fn an_image_packed_with_libraries_is_reached_through_a_launcher() {
    let mut fixture = grilling().await;

    // An AppDir as `tools/build-appimage.sh` packs one: the image under
    // `usr/bin`, and in `usr/lib` beside it a file standing in for the
    // libraries — what a test can ask of a loader path is what is on it, and a
    // real ELF would prove no more than a name does.
    let appdir = fixture.state.path().join("appdir");
    let bin = appdir.join("usr/bin");
    let lib = appdir.join("usr/lib");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(lib.join("libpacked-with-it.so.0"), "not really an ELF\n").unwrap();

    // Which says what it was given to load over as well as which build it is:
    // the launcher is what put that there, and the image is where it can be
    // read back.
    let image = bin.join("verkstead");
    std::fs::write(
        &image,
        "#!/bin/sh\nprintf 'the-servers-own over %s\\n' \"${LD_LIBRARY_PATH-nothing}\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o755)).unwrap();

    fixture.verkstead = Executable::at(Platform::HERE, image, fixture.state.path())
        .expect("the image was just written")
        .bundling(fixture.state.path(), Some(&appdir));

    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        r#"
        found=$(command -v verkstead)
        say found "$found"
        say ran "$(verkstead)"
        say beside "$(ls "$(dirname "$found")")"
        say packed "$(ls /verkstead/lib)"
        say session-loader "${LD_LIBRARY_PATH-nothing}"
        "#,
    );

    assert_eq!(
        reported["found"], "/verkstead/bin/verkstead",
        "a bare `verkstead` is still the one thing on the server's own PATH entry"
    );
    assert_eq!(
        reported["ran"], "the-servers-own over /verkstead/lib",
        "and running it runs the server's own image, over the libraries it was packed with"
    );
    assert_eq!(
        reported["beside"], "verkstead",
        "the image itself is behind the launcher rather than beside it on the PATH"
    );
    assert_eq!(
        reported["packed"], "libpacked-with-it.so.0",
        "the libraries a session gets are the ones the image was packed with"
    );
    assert_eq!(
        reported["session-loader"], "nothing",
        "and the session's own environment says nothing to the loader, so nothing else \
         it runs loads out of that directory"
    );
}

/// What `SHELL` says inside: `/bin/sh` for a session, and for a Conversation's
/// own terminal the shell the human is actually typing into.
///
/// A session has no shell of its own — it runs an agent, and the variable is
/// there for whatever the agent shells out with — so the certain path is the
/// right answer for one. A terminal *is* a shell, and one whose `SHELL` named
/// something else would be a terminal where every tool that starts a shell
/// started a different one from the one at the keyboard.
#[tokio::test]
async fn the_shell_a_sandbox_names_is_the_one_it_was_built_to_run() {
    let fixture = grilling().await;

    let session = probe(&fixture.sandbox(vec![]), r#"say shell "${SHELL-unset}""#);

    assert_eq!(
        session["shell"], "/bin/sh",
        "a session's is the one path every platform is certain to have a shell at"
    );

    // Somewhere a shell really is on this machine, which is the shape a passwd
    // entry names — the choosing of one is `terminals::shell`'s own business and
    // is asked there.
    let chosen = on_the_host("bash");

    let terminal = probe(
        &fixture
            .sandbox(vec![])
            .shelled(&chosen.display().to_string()),
        r#"say shell "${SHELL-unset}""#,
    );

    assert_eq!(
        terminal["shell"],
        chosen.display().to_string(),
        "and a terminal's is the shell it was built to run"
    );
}

#[tokio::test]
async fn no_other_checkout_on_the_machine_is_reachable() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {sibling} sibling
            file {readme} repo-readme
            say outside-holds "$(ls -A {elsewhere} | sort | tr '\n' ' ')"
            say repo-holds "$(ls -A {repo} | sort | tr '\n' ' ')"
            "#,
            elsewhere = quoted(fixture.elsewhere.path()),
            repo = quoted(&fixture.repo),
            sibling = quoted(&fixture.sibling),
            readme = quoted(&fixture.repo.join("README.md")),
        ),
    );

    assert_eq!(
        reported["sibling"], "absent",
        "another repository in the same directory is another Conversation's business"
    );
    assert_eq!(
        reported["repo-readme"], "absent",
        "the checkout the worktree was made from is not the worktree"
    );

    // A bind's parent directories have to exist for it to land on, so the
    // directory the repository is in is inside as a scaffold: empty tmpfs
    // directories holding nothing but what was deliberately bound, and writing
    // in them writes nothing the host will ever see.
    assert_eq!(
        reported["outside-holds"], "verkstead ",
        "nothing around the Repo arrives except by being bound — and the \
         Profile's pair arrives in HOME rather than where it lives"
    );
    assert_eq!(
        reported["repo-holds"], ".git ",
        "the Repo is inside as its git directory and nothing else"
    );
}

#[tokio::test]
async fn the_network_is_the_hosts_own() {
    let fixture = grilling().await;
    let sandbox = fixture.sandbox(vec![]);

    // A listener in this process, which is in the host's network namespace. A
    // sandbox with a namespace of its own would have its own empty loopback and
    // find nothing at this port — so reaching it is the sharing, proved without
    // anything here touching the internet.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let answering = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("the probe connects");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
            .unwrap();
    });

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            if {curl} --silent --max-time 10 --output /dev/null "http://127.0.0.1:{port}/"; then
                say network reached
            else
                say network unreachable
            fi
            "#,
            curl = quoted(&on_the_host("curl")),
        ),
    );

    assert_eq!(
        reported["network"], "reached",
        "the filesystem is the boundary; the network is not"
    );

    answering.join().unwrap();
}

/// The one thing a session is given that is not a directory, and the whole of
/// what makes its Question Sets its own Conversation's.
///
/// Asked from inside rather than read off the flags the sandbox was built with,
/// like every other claim in this file: what settles whether a session can reach
/// Verkstead is a session trying to, and a Set that lands on the right Timeline
/// is the only evidence that it did.
///
/// The server is real and listening on the host's loopback, which the sandbox
/// shares — see [`the_network_is_the_hosts_own`]. Everything but the agent is
/// real too: `curl` inside the sandbox is standing in for the bundled CLI, which
/// posts exactly this to exactly this URL.
#[tokio::test]
async fn a_session_puts_a_set_to_its_own_conversation_and_nothing_else() {
    let fixture = grilling().await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let listening = listener.local_addr().unwrap();

    let serving = tokio::spawn({
        let pool = fixture.pool.clone();
        async move {
            let _ = axum::serve(listener, verkstead_server::router(pool)).await;
        }
    });

    let sandbox = fixture.sandbox_reaching(listening, &BuildCache::none(), vec![]);

    // Every part of the sandbox blocks, and this one is a process talking to a
    // server on the runtime this test is on.
    let reported = tokio::task::spawn_blocking(move || {
        probe(
            &sandbox,
            &format!(
                r#"
                say server "$VERKSTEAD_SERVER"

                {curl} --silent --show-error --fail \
                    --header 'Content-Type: application/yaml' \
                    --data-binary @- \
                    --output /tmp/created \
                    "$VERKSTEAD_SERVER/api/v1/sets" <<'YAML'
title: What a delivery that has failed forty times becomes
questions:
  - label: Q1
    text: How many failures before an endpoint is given up on?
    options:
      - n: 1
        text: Five
        recommended: true
YAML

                say submitted "$?"
                "#,
                curl = quoted(&on_the_host("curl")),
            ),
        )
    })
    .await
    .unwrap();

    assert_eq!(
        reported["server"],
        format!(
            "http://{listening}/conversations/{}",
            fixture.conversation.id
        ),
        "a session is pointed at its own Conversation, explicitly"
    );
    assert_eq!(
        reported["submitted"], "0",
        "the server should have taken the Set"
    );

    let timeline = store::timeline(&fixture.pool, fixture.conversation.id)
        .await
        .unwrap();

    let asked: Vec<&store::SetOnTimeline> = timeline
        .iter()
        .filter_map(|event| match &event.event {
            store::Event::QuestionSet(asked) => Some(asked.as_ref()),
            _ => None,
        })
        .collect();

    assert_eq!(asked.len(), 1, "the Timeline it landed on is its own");
    assert_eq!(
        asked[0]
            .set
            .set()
            .expect("the Set the session just sent reads back")
            .title,
        "What a delivery that has failed forty times becomes"
    );

    serving.abort();
}

/// And the half of the same loopback a session may *not* reach: the workbench's
/// own namespace, which answers 401 to everything that has not shown the
/// Workbench Key (ADR-0015).
///
/// A session's network is the host's own — see
/// [`the_network_is_the_hosts_own`] — so the address the agent contract is
/// served on is the address the workbench is served on, and nothing about the
/// socket tells the two apart. What does is a secret in the Data Directory,
/// which is not among the things a sandbox binds.
///
/// Asked from inside rather than read off the flags, like every other claim in
/// this file: what settles whether a session can register a Repo through the
/// viewer's API is a session trying to, and the status it reads back.
#[tokio::test]
async fn a_session_is_refused_the_workbenchs_own_namespace() {
    let fixture = grilling().await;

    // In the Data Directory, where a real server keeps it.
    let key = WorkbenchKey::issued(fixture.state.path()).unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let listening = listener.local_addr().unwrap();

    let serving = tokio::spawn({
        let pool = fixture.pool.clone();
        let key = key.clone();
        async move {
            let _ = axum::serve(listener, verkstead_server::router_keyed(pool, key)).await;
        }
    });

    let sandbox = fixture.sandbox_reaching(listening, &BuildCache::none(), vec![]);
    let key_file = key.path().to_owned();

    let reported = tokio::task::spawn_blocking(move || {
        probe(
            &sandbox,
            &format!(
                r#"
                status() {{
                    {curl} --silent --output /dev/null --write-out '%{{http_code}}' "$2" \
                        > /tmp/status 2>/dev/null
                    say "$1" "$(cat /tmp/status)"
                }}

                status workbench "http://{listening}/api/ui/repos"
                status page "http://{listening}/conversations"
                status health "http://{listening}/api/v1/health"

                file {key_file} key-file
                "#,
                curl = quoted(&on_the_host("curl")),
                key_file = quoted(&key_file),
            ),
        )
    })
    .await
    .unwrap();

    assert_eq!(
        reported["workbench"], "401",
        "a session shares the loopback, so what keeps it out of the viewer's \
         namespace is the key rather than the network"
    );
    assert_eq!(
        reported["page"], "401",
        "and out of the workbench's own pages, which carry the same"
    );
    assert_eq!(
        reported["health"], "200",
        "whether the server is up is not a question about anybody's work"
    );
    assert_eq!(
        reported["key-file"], "absent",
        "the key is in the Data Directory, which no sandbox mounts"
    );

    serving.abort();
}

#[tokio::test]
async fn the_extra_binds_sandbox_configuration_asks_for_are_there_and_writable() {
    let fixture = grilling().await;

    // Two of them, read off the configuration as the orchestrator will read
    // them: every session gets every one.
    let cache = fixture.state.path().join("shared-cache");
    let cargo = fixture.state.path().join("shared-cargo-home");
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::create_dir_all(&cargo).unwrap();

    let config =
        SandboxConfig::resolve(&[cache.display().to_string(), cargo.display().to_string()])
            .unwrap();

    let sandbox = fixture.sandbox(config.binds());

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {cache} cache
            dir {cargo} cargo
            dir {other} unconfigured
            "#,
            cache = quoted(&cache),
            cargo = quoted(&cargo),
            other = quoted(&fixture.sibling),
        ),
    );

    assert_eq!(
        reported["cache"], "write",
        "what the machine gives every session"
    );
    assert_eq!(reported["cargo"], "write", "and the one beside it");
    assert_eq!(
        reported["unconfigured"], "absent",
        "and a directory nobody configured is no part of a sandbox"
    );
}

/// A read-only companion is there to be read: the checkout and the history
/// behind it, and nothing a session does inside it changes either.
///
/// Both halves are asked of git rather than of the flags. A worktree bound
/// read-only with a writable git directory would still take a commit, and one
/// whose git directory is read-only refuses more than committing — the index and
/// its lock live in there too, so what has to be shown is that reading really
/// does work rather than that writing really does not.
#[tokio::test]
async fn a_read_only_companion_is_there_to_read_and_a_commit_from_it_is_refused() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadOnly)]).await;
    fixture.configure("git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n");

    let companion = fixture.companion_worktree("askance").to_owned();

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            dir {companion} worktree
            dir {git_dir} git-dir
            say readme "$({cat} {companion}/README.md)"
            say history "$({git} -C {companion} log --oneline | wc -l)"

            if {git} -C {companion} status --porcelain >/dev/null 2>&1; then
                say status yes
            else
                say status no
            fi

            if {git} -C {companion} commit --quiet --allow-empty -m 'from inside' 2>/dev/null; then
                say committed yes
            else
                say committed no
            fi

            if {git} -C {companion} branch verkstead-probe 2>/dev/null; then
                say branched yes
            else
                say branched no
            fi
            "#,
            companion = quoted(&companion),
            git_dir = quoted(&fixture.companion("askance").repo.path.join(".git")),
            cat = quoted(&on_the_host("cat")),
            git = quoted(&on_the_host("git")),
        ),
    );

    assert_eq!(
        reported["worktree"], "read",
        "a read-only companion is a checkout to read and leave alone"
    );
    assert_eq!(
        reported["git-dir"], "read",
        "and the object database behind it, which is what makes it a repository at all"
    );
    assert_eq!(
        reported["readme"], "# a repository",
        "reading a file in one really works"
    );
    assert_eq!(
        reported["history"], "1",
        "and so does asking git what the history is, which is the half worth \
         proving rather than assuming"
    );
    assert_eq!(
        reported["status"], "yes",
        "and so does the question an agent asks a checkout first of all, which \
         git answers without writing the index it would rather refresh"
    );
    assert_eq!(
        reported["committed"], "no",
        "there is nowhere for a commit to be written"
    );
    assert_eq!(
        reported["branched"], "no",
        "and no ref can be moved, which is what the last step of a push is"
    );
}

/// A read-write companion is somewhere the work is done: a commit lands on the
/// branch that was cut for it, and it is there when the session has gone.
#[tokio::test]
async fn a_read_write_companion_takes_a_commit_on_its_own_branch() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadWrite)]).await;
    fixture.configure("git_author:\n  name: Tobias Cohen\n  email: tobi@tobico.net\n");

    let companion = fixture.companion_worktree("askance").to_owned();

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            printf 'from inside\n' > {companion}/NOTES.md
            {git} -C {companion} add NOTES.md

            if {git} -C {companion} commit --quiet -m 'from inside' 2>/tmp/git-said; then
                say committed yes
            else
                say committed "no: $(cat /tmp/git-said)"
            fi
            "#,
            companion = quoted(&companion),
            git = quoted(&on_the_host("git")),
        ),
    );

    assert_eq!(reported["committed"], "yes");

    // Read outside, where the branch the checkout was cut on is: what a session
    // committed in a companion is in that companion's repository, on the branch
    // the Conversation's own name was mirrored into.
    assert_eq!(
        git(&companion, &["log", "-1", "--format=%s"]).trim(),
        "from inside"
    );
    assert_eq!(
        git(&companion, &["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
        "rate-limiting",
        "a read-write companion is cut a branch of its own, mirroring the \
         Conversation's where nothing else was typed"
    );
}

/// What Sandbox Configuration asks for is inside and writable beside a
/// read-only companion's checkout, whatever the companion's mode.
///
/// A build cache is a hole somebody opened on purpose, and it sits outside every
/// repository: a read-only companion is a checkout not to be changed rather
/// than a repository whose builds should fail on a cold cache.
#[tokio::test]
async fn the_configured_binds_beside_a_read_only_companion_are_still_writable() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadOnly)]).await;

    let cache = fixture.state.path().join("node-modules");
    std::fs::create_dir_all(&cache).unwrap();

    let config = SandboxConfig::resolve(&[cache.display().to_string()]).unwrap();
    let sandbox = fixture.sandbox(config.binds());

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {cache} cache
            dir {companion} worktree
            "#,
            cache = quoted(&cache),
            companion = quoted(fixture.companion_worktree("askance")),
        ),
    );

    assert_eq!(
        reported["cache"], "write",
        "a companion's builds need the machine's caches like any other repository's"
    );
    assert_eq!(
        reported["worktree"], "read",
        "and the checkout beside it is still only there to be read"
    );
}

/// The set itself, without a sandbox to run in: one list, in the order it was
/// configured, and the same list whatever Conversation is asking.
///
/// A Repo could once ask for a set of its own, composed over this one, and that
/// is gone: a bind is a build cache or a package registry, which is the
/// machine's rather than one repository's.
#[tokio::test]
async fn every_configured_bind_is_one_every_sandbox_gets() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let cargo = dir.path().join("cargo");
    let node = dir.path().join("node");
    for made in [&cache, &cargo, &node] {
        std::fs::create_dir(made).unwrap();
    }

    let config = SandboxConfig::resolve(&[
        cache.display().to_string(),
        cargo.display().to_string(),
        node.display().to_string(),
    ])
    .unwrap();

    assert_eq!(
        config.binds(),
        vec![
            Bind::writable(cache),
            Bind::writable(cargo),
            Bind::writable(node),
        ],
        "every configured bind, in the order it was configured"
    );
}

/// The same configuration said in `config.yaml` instead: the binds are inside,
/// they are writable, and they are added to what the installation configured
/// rather than standing in for it.
///
/// Which is the whole of what a standalone install needs — a bare binary whose
/// sandbox is set up from the settings page, with no flag and no environment
/// variable in front of it.
#[tokio::test]
async fn the_binds_the_settings_file_holds_are_inside_and_writable_too() {
    let fixture = grilling().await;

    let installed = fixture.state.path().join("the-installations-cache");
    let said = fixture.state.path().join("the-settings-cache");
    let scoped = fixture.state.path().join("the-settings-verkstead-cargo");
    for made in [&installed, &said, &scoped] {
        std::fs::create_dir_all(made).unwrap();
    }

    // And an entry in the retired `name=path` grammar, which reaches no session
    // however it is spelled: the directory is really there, and a session that
    // got it would be a session getting a setting nobody can see on the page.
    fixture.configure(&format!(
        "sandbox_binds:\n  - {said}\n  - verkstead={scoped}\n",
        said = said.display(),
        scoped = scoped.display(),
    ));

    // And one on the command line beside them, which is the composition worth
    // asking about: neither set is the other's replacement.
    let installation = SandboxConfig::resolve(&[installed.display().to_string()]).unwrap();
    let sandbox = fixture.sandbox(installation.binds());

    let reported = probe(
        &sandbox,
        &format!(
            r#"
            dir {installed} installed
            dir {said} said
            dir {scoped} scoped
            "#,
            installed = quoted(&installed),
            said = quoted(&said),
            scoped = quoted(&scoped),
        ),
    );

    assert_eq!(
        reported["installed"], "write",
        "what the installation configured is still there"
    );
    assert_eq!(
        reported["said"], "write",
        "and what the settings file adds to it"
    );
    assert_eq!(
        reported["scoped"], "absent",
        "and an entry written for one Repo reaches nobody at all"
    );
}

/// A settings-held bind naming a directory that is not there is skipped, and the
/// session starts without it.
///
/// The opposite answer to the flag's, and deliberately: a file edited from a
/// phone is a file with typos in it, and a Conversation that would not start
/// because one of its binds was misspelled is a worse failure than a build
/// running without its cache. The line in the log is what says which.
#[tokio::test]
async fn a_settings_bind_that_is_not_there_is_skipped_and_the_session_still_starts() {
    let fixture = grilling().await;

    let there = fixture.state.path().join("the-cache-that-is-there");
    std::fs::create_dir_all(&there).unwrap();
    let missing = fixture.state.path().join("never-made");

    fixture.configure(&format!(
        "sandbox_binds:\n  - {missing}\n  - {there}\n",
        missing = missing.display(),
        there = there.display(),
    ));

    let reported = probe(
        &fixture.sandbox(vec![]),
        &format!(
            r#"
            dir {there} there
            dir {missing} missing
            "#,
            there = quoted(&there),
            missing = quoted(&missing),
        ),
    );

    assert_eq!(
        reported["there"], "write",
        "the bind beside the one that would not resolve is still handed over"
    );
    assert_eq!(
        reported["missing"], "absent",
        "and the one that would not resolve is simply not there"
    );
}

/// What the settings-held set comes to for a Conversation, without a sandbox to
/// run in: every entry, in the order it was written — the one set the
/// installation's own is — and everything that is not one taken out of it.
///
/// The Conversation has a companion, which is the case worth asking about: a
/// companion brings its checkout, and what it builds with is the set every
/// session gets rather than anything of its own.
#[tokio::test]
async fn the_settings_held_binds_compose_the_way_the_installations_do() {
    let fixture = grilling_alongside(&[("askance", store::CompanionMode::ReadOnly)]).await;

    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let cargo = dir.path().join("cargo");
    let scoped = dir.path().join("scoped");
    for made in [&cache, &cargo, &scoped] {
        std::fs::create_dir(made).unwrap();
    }

    let binds = [
        cache.display().to_string(),
        cargo.display().to_string(),
        // The retired grammar, dropped without a word however it is spelled:
        // the Conversation's own Repo, its companion's, and a name nothing at
        // all is registered under.
        format!("verkstead={}", scoped.display()),
        format!("askance={}", scoped.display()),
        format!("something-nobody-added={}", scoped.display()),
        // And the three shapes that go in the log instead of into the sandbox:
        // a directory nobody made, a path that is not one, and a `=` with no
        // name in front of it.
        dir.path().join("never-made").display().to_string(),
        "relative/cache".to_owned(),
        "=/var/cache".to_owned(),
    ];

    assert_eq!(
        SandboxConfig::settings_binds(&binds, &fixture.conversation),
        vec![Bind::writable(cache), Bind::writable(cargo)],
        "every entry that is an absolute path and is there, and nothing else"
    );
}

/// The shared Rust build cache, with nothing configured — which is the feature
/// on, because a human who has never opened the settings page should not be the
/// one paying for every dependency to be compiled twice.
///
/// The directory is writable at the same path inside, and `CARGO_HOME` points
/// into it: that is the half of the cache that works with no sccache anywhere,
/// and it is what stops two Conversations downloading one crate twice.
#[tokio::test]
async fn the_build_cache_is_writable_inside_and_cargos_home_is_in_it() {
    let fixture = grilling().await;
    let cache = fixture.cache(false);

    let reported = probe(
        &fixture.sandbox_caching(&cache),
        &format!(
            r#"
            dir {dir} cache
            say cargo-home "${{CARGO_HOME-unset}}"
            say wrapper "${{RUSTC_WRAPPER-unset}}"
            say sccache-dir "${{SCCACHE_DIR-unset}}"
            "#,
            dir = quoted(&fixture.cache_dir()),
        ),
    );

    assert_eq!(
        reported["cache"], "write",
        "a cache a session cannot write to is no cache"
    );
    assert_eq!(
        reported["cargo-home"],
        fixture.cache_dir().join("cargo").display().to_string(),
        "the registry every session downloads into is the one inside the bind"
    );
    assert_eq!(
        reported["wrapper"], "unset",
        "with no sccache resolved there is nothing to wrap rustc in, and a \
         RUSTC_WRAPPER naming a path that is not mounted would break every build"
    );
    assert_eq!(reported["sccache-dir"], "unset");
}

/// And with an sccache the server resolved: it is mounted beside the
/// `verkstead` binary, it is *that* file rather than whatever the machine has
/// installed, and it is what `RUSTC_WRAPPER` names.
///
/// The wrapper is named absolutely on purpose. A session's command may be
/// wrapped in `nix develop`, which puts the project's own shell in front of the
/// sandbox's `PATH` — so a bare `sccache` would resolve to whatever that shell
/// had, or to nothing.
#[tokio::test]
async fn the_sccache_the_server_resolved_is_what_rustc_is_wrapped_in() {
    let fixture = grilling().await;
    let cache = fixture.cache(true);

    let reported = probe(
        &fixture.sandbox_caching(&cache),
        &format!(
            r#"
            say wrapper "${{RUSTC_WRAPPER-unset}}"
            say sccache-dir "${{SCCACHE_DIR-unset}}"
            say size "${{SCCACHE_CACHE_SIZE-unset}}"
            say which "$("${{RUSTC_WRAPPER}}")"
            dir {dir} cache
            "#,
            dir = quoted(&fixture.cache_dir()),
        ),
    );

    assert_eq!(
        reported["wrapper"], "/verkstead/bin/sccache",
        "absolute, because a project's dev shell decides what `PATH` holds"
    );
    assert_eq!(
        reported["which"], "sccache 0.0.0-the-one-resolved",
        "what a session compiles through is the binary the server resolved"
    );
    assert_eq!(
        reported["sccache-dir"],
        fixture.cache_dir().join("sccache").display().to_string(),
        "and it writes its objects inside the one bind, beside cargo's own"
    );
    assert_eq!(
        reported["size"], "30G",
        "the default where the human has configured no size"
    );
    assert_eq!(reported["cache"], "write");
}

/// The switch is the human's, in `config.yaml` and on the settings page, and it
/// is read as each sandbox is built — so turning it off is a next session with
/// no bind and none of the variables.
#[tokio::test]
async fn a_build_cache_switched_off_is_no_bind_and_no_variables() {
    let fixture = grilling().await;
    fixture.configure("rust_build_cache:\n  enabled: false\n");

    // The server still resolved one, sccache and all: what is being shown is
    // that the switch decides, not that there was nothing to hand out.
    let cache = fixture.cache(true);

    let reported = probe(
        &fixture.sandbox_caching(&cache),
        &format!(
            r#"
            dir {dir} cache
            say cargo-home "${{CARGO_HOME-unset}}"
            say wrapper "${{RUSTC_WRAPPER-unset}}"
            say sccache-dir "${{SCCACHE_DIR-unset}}"
            say size "${{SCCACHE_CACHE_SIZE-unset}}"
            file /verkstead/bin/sccache binary
            "#,
            dir = quoted(&fixture.cache_dir()),
        ),
    );

    assert_eq!(
        reported["cache"], "absent",
        "the switch closes the hole rather than leaving it open and unused"
    );
    assert_eq!(reported["cargo-home"], "unset");
    assert_eq!(reported["wrapper"], "unset");
    assert_eq!(reported["sccache-dir"], "unset");
    assert_eq!(reported["size"], "unset");
    assert_eq!(
        reported["binary"], "absent",
        "and the sccache goes with it: there is nothing left for it to compile into"
    );
}

/// The compile server: Verkstead's own, in a sandbox holding the Worktrees
/// directory and the cache and nothing else Verkstead keeps.
///
/// This is what stops two Conversations building Rust at once from breaking
/// each other. An sccache server is what executes `rustc` — the client in a
/// sandbox only hands it a command line — and every sandbox shares the host's
/// network, so clients left to start their own all reach for one port and the
/// session that lost the race has its compiles run inside the winner's sandbox,
/// where its Worktree is not bound and the build fails outright.
///
/// So the server has to see **every** Worktree, which is why it gets the
/// directory rather than any one of them — a Conversation grilled after it
/// started is one it can already compile for. And it must not see the rest of
/// the Data Directory: `rustc` runs proc macros while it compiles, so a server
/// with the database and the settings files in reach would be every Rust
/// dependency on the machine holding the GitHub token.
#[tokio::test]
async fn the_compile_server_holds_the_worktrees_and_none_of_the_data_directory() {
    let fixture = grilling().await;

    // The four things it must not reach, really on disk so that their absence
    // inside is the bind rather than the fixture.
    fixture.configure("git_author:\n  name: Tobias Cohen\n");
    fixture.configure_github_token("github_token: ghp_thetoken\n");
    std::fs::write(fixture.state.path().join("verkstead.db"), "the database\n").unwrap();
    fixture.attach("wireframe.png", b"PNG");

    let cache = fixture.cache(true);
    cache.compiling(&RustBuildCache::default(), None);

    let reported = compile_server_report(&fixture);

    assert_eq!(
        reported["worktrees"], "write",
        "every Conversation's checkout, writable, because a compile writes its \
         output into the Worktree's own target/"
    );
    assert_eq!(
        reported["cache"], "write",
        "and the cache it reads its dependency sources out of and writes its \
         objects into"
    );

    assert_eq!(
        reported["database"], "absent",
        "the database is not the compile server's, and a proc macro compiles as \
         whoever Verkstead runs as"
    );
    assert_eq!(reported["config"], "absent");
    assert_eq!(
        reported["secrets"], "absent",
        "least of all the file the GitHub token is in"
    );
    assert_eq!(
        reported["handoffs"], "absent",
        "and nothing else of the Data Directory either — the Worktrees are the \
         whole of the bind"
    );
    assert_eq!(
        reported["attachments"], "absent",
        "the files the human attached are a session's to read, and the compile \
         server is not a session"
    );

    assert_eq!(
        reported["size"], "30G",
        "started with the size the human left, which sccache reads once"
    );
    assert_eq!(
        reported["no-daemon"], "1",
        "in the foreground, so it is a child Verkstead holds rather than a \
         daemon nothing can ask about"
    );
    assert_eq!(
        reported["idle"], "0",
        "and it does not time out: an unattended Conversation may go a long \
         while between builds"
    );
    assert_eq!(
        reported["sccache-dir"],
        fixture.cache_dir().join("sccache").display().to_string(),
        "writing into the same objects a session's own fallback server would"
    );
    assert_eq!(reported["home"], "/verkstead/home");
}

/// One compile server and no more, however many sessions ask for one — which is
/// the whole of the arrangement: two servers would be two ports and one of them
/// unreachable.
#[tokio::test]
async fn a_second_session_asking_for_a_compile_server_gets_the_one_already_up() {
    let fixture = grilling().await;
    let cache = fixture.cache(true);

    cache.compiling(&RustBuildCache::default(), None);

    let first = compile_server_report(&fixture);
    let started = std::fs::metadata(fixture.cache_dir().join(COMPILE_SERVER_REPORT))
        .unwrap()
        .modified()
        .unwrap();

    // The clone a session's spawn is handed, which is the one that would start a
    // second server if this were held per session rather than per machine.
    cache.clone().compiling(&RustBuildCache::default(), None);

    assert_eq!(
        std::fs::metadata(fixture.cache_dir().join(COMPILE_SERVER_REPORT))
            .unwrap()
            .modified()
            .unwrap(),
        started,
        "nothing wrote the report again, so nothing started a second server: {first:?}",
    );
}

/// The size sccache is told is read once, when its server starts — so changing
/// it in the workbench starts the server again rather than saving a number
/// nothing ever reads.
#[tokio::test]
async fn a_size_the_human_changed_starts_the_compile_server_again() {
    let fixture = grilling().await;
    let cache = fixture.cache(true);

    cache.compiling(&RustBuildCache::default(), None);
    assert_eq!(compile_server_report(&fixture)["size"], "30G");

    std::fs::remove_file(fixture.cache_dir().join(COMPILE_SERVER_REPORT)).unwrap();
    cache.compiling(&RustBuildCache::of(true, Some("5G".to_owned())), None);

    assert_eq!(
        compile_server_report(&fixture)["size"],
        "5G",
        "the server that is up is the one told the size the human just typed"
    );
}

/// What the compile server wrote down about its own sandbox, waited for.
///
/// Waited rather than read, because starting it is a `spawn` and what is being
/// read is a file the child writes: the alternative is a test that passes or
/// fails on how busy the machine was.
fn compile_server_report(fixture: &Grilling) -> BTreeMap<String, String> {
    let report = fixture.cache_dir().join(COMPILE_SERVER_REPORT);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(20);

    while std::time::Instant::now() < until {
        // Written whole and then read, so a report caught half-written is one
        // more go round rather than a missing key: the last line is the one
        // every caller asserts on.
        if let Ok(said) = std::fs::read_to_string(&report)
            && said.contains("secrets=")
        {
            return said
                .lines()
                .filter_map(|line| line.split_once('='))
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect();
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    panic!(
        "the compile server never wrote its report to {}",
        report.display()
    );
}

/// The size is the human's word for one, handed to sccache as it was written —
/// nothing here parses it, because what sccache makes of a size is sccache's to
/// say.
#[tokio::test]
async fn the_size_the_human_configured_is_what_sccache_is_told() {
    let fixture = grilling().await;
    fixture.configure("rust_build_cache:\n  size: 5G\n");

    let reported = probe(
        &fixture.sandbox_caching(&fixture.cache(true)),
        r#"say size "${SCCACHE_CACHE_SIZE-unset}""#,
    );

    assert_eq!(reported["size"], "5G");
}

#[test]
fn a_bind_that_is_not_there_refuses_to_resolve() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("never-made");

    assert!(
        SandboxConfig::resolve(&[missing.display().to_string()]).is_err(),
        "a bind bwrap could not make is every session failing to start"
    );
}

/// Every entry that is not an absolute path, the retired `name=path` among them.
///
/// The flag is where a bad entry is heard about: a service unit is started once,
/// nobody is watching when it is wrong, and a startup that refused by name is
/// how somebody learns their `name=path` is a grammar Verkstead no longer reads.
#[test]
fn a_bind_that_is_not_an_absolute_path_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let there = dir.path().display().to_string();

    for refused in [
        "cache".to_owned(),
        "verkstead=cache".to_owned(),
        "=/var/cache".to_owned(),
        format!("verkstead={there}"),
    ] {
        let error = match SandboxConfig::resolve(std::slice::from_ref(&refused)) {
            Ok(_) => panic!("{refused:?} should be refused"),
            Err(error) => error.to_string(),
        };

        assert!(
            error.contains(&refused),
            "the entry itself is what the refusal names: {error}"
        );
    }
}

/// A path as one shell word.
///
/// Single quotes, because a temporary directory's name is not the test's to
/// choose and a space in `TMPDIR` would otherwise split the probe's argument in
/// two.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

#[test]
fn a_repository_whose_flake_has_a_dev_shell_runs_its_command_under_nix_develop() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("flake.nix"), DEV_SHELL).unwrap();

    assert_eq!(
        under_dev_shell(Platform::HERE, dir.path(), &["claude".to_owned()]),
        vec![
            "nix".to_owned(),
            "develop".to_owned(),
            "--command".to_owned(),
            "claude".to_owned()
        ],
    );
}

#[test]
fn a_flake_that_defines_no_shell_runs_the_command_as_it_stands() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("flake.nix"), NO_DEV_SHELL).unwrap();

    assert_eq!(
        under_dev_shell(Platform::HERE, dir.path(), &["claude".to_owned()]),
        vec!["claude".to_owned()],
        "`nix develop` errors out where none of the attributes it falls through exist"
    );
}

#[test]
fn a_repository_with_no_flake_at_all_runs_the_command_as_it_stands() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(
        under_dev_shell(Platform::HERE, dir.path(), &["claude".to_owned()]),
        vec!["claude".to_owned()],
    );
}

/// And a Windows session is never put under one, whatever the worktree holds.
///
/// The same worktree the test above this one wraps: a flake that really does
/// define a dev shell, which is the one case the answer could have come from
/// asking. So an unwrapped command here is an answer that was never asked for
/// — there is no `nix` on that machine to ask, and a session should not pay a
/// process to find out.
#[test]
fn a_windows_session_is_never_put_under_a_dev_shell() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("flake.nix"), DEV_SHELL).unwrap();

    assert_eq!(
        under_dev_shell(Platform::Windows, dir.path(), &["claude".to_owned()]),
        vec!["claude".to_owned()],
    );
}

/// A flake with a dev shell and no inputs, so the evaluation this provokes needs
/// nothing fetched and no store path built.
const DEV_SHELL: &str = r#"
{
  outputs = { self }: {
    devShells.x86_64-linux.default = "a dev shell";
    devShells.aarch64-linux.default = "a dev shell";
  };
}
"#;

/// And one that builds a package under a name `nix develop` does not fall
/// through to.
const NO_DEV_SHELL: &str = r#"
{
  outputs = { self }: {
    packages.x86_64-linux.something = "not a shell";
    packages.aarch64-linux.something = "not a shell";
  };
}
"#;
