//! **The account mirror**: a session on one device running under an Agent
//! Profile whose account lives on another (ADR-0020, *Shared Profiles*).
//!
//! **Two Verksteads, a real dial and a real session.** `tests/mirroring.rs` is
//! the row written down over the link; this is the account itself coming across
//! it. So nothing here hands B a login by the back door: A's account is a real
//! `.claude` on disk with a real login in it, B reads it over the Peer Listener
//! behind the Member Gate, writes the mirror under its own Data Directory, and
//! the session that runs is a real session in a real sandbox reading the root it
//! was given.
//!
//! **What stands in for claude is a shell script**, for `tests/sessions.rs`'s
//! reason: what is being asked is what a session away from home is *given*, and
//! asking it of the real claude would be a test that needed an account, a network
//! and a model's patience. What the stub does is print the files it finds in its
//! root, which is the only thing this suite wants to know.
//!
//! **And the fetch itself is read directly** for the questions a whole session
//! would only answer at one remove: which login a second launch gets, and what
//! landed in the mirror. That is the reading a launch makes — see
//! `mirroring::account::fetched` — asked where the device about to launch asks it.
//!
//! **On the machine this suite is about**: the sandbox is bwrap and the terminal
//! is a real pseudo-terminal, which is the arrangement `tests/sessions.rs` is
//! written for and the reason that file is Unix-only too.
#![cfg(unix)]

use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{Capture, ConversationView, ProfileEntry, Started, TimelineEvent};
use verkstead_server::attachments::Attachments;
use verkstead_server::build_cache::BuildCache;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::handoffs::Handoffs;
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Environment, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::sandbox::{Executable, Homes, Reachable, SandboxConfig};
use verkstead_server::settings::Settings;
use verkstead_server::skills::Skills;
use verkstead_server::{
    Agents, Gh, Refusal, Routers, mirroring, open_database, routers_answering_devices_telling,
    routers_running_sessions_answering_devices, store,
};
use verkstead_store::{Linking, record_member};

/// The device the account is at home on.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the device the session runs on, which is where every assertion is made.
const B: &str = "0011223344556677889900aabbccddee";

/// What A is written down as on B: the name the human gave the machine, which is
/// what a refusal names rather than sixteen bytes of hex.
const A_MACHINE: &str = "the-workstation";
const A_OS: &str = "macOS 15.1";

/// The Profiles section and every pairing picker, which is where a mirror is read
/// off.
const PROFILES: &str = "/api/ui/profiles";

/// The model every Profile here lists.
const MODEL: &str = "claude-opus-5";

/// What the account on A is signed in with, and what a session on B has to have
/// in its root for this to have worked at all.
const TOKEN: &str = "sk-ant-oat01-the-workstations-own";

/// And what it is signed in with after a refresh at home.
const REFRESHED: &str = "sk-ant-oat01-refreshed-at-home";

/// The Brief every Conversation here is started from.
const BRIEF: &str = "# Rate limiting\n\nThe API has none.\n";

/// What stands where claude goes: a session that prints the three files of its
/// root and stops.
///
/// Read out of the root as the session finds it inside the sandbox, which is the
/// whole question — a login joined in from the mirror is a session that is logged
/// in, and a settings file with the account's own hooks in it would be a mirror
/// carrying what a root never carries.
const PRINTS_ITS_ROOT: &str = r#"
printf 'login: %s\n' "$(cat "$HOME/.claude/.credentials.json" 2>&1)"
printf 'settings: %s\n' "$(cat "$HOME/.claude/settings.json" 2>&1)"
printf 'config: %s\n' "$(cat "$HOME/.claude.json" 2>&1)"
"#;

/// What stands where claude goes when the question is the **memory sync**: a
/// session that reads out everything its store holds, writes a transcript into
/// the entry for the directory it is working in, and a memory into the other
/// entry it was given.
///
/// **The entry name is computed inside**, out of the directory the session was
/// started in and by the harness's own rule — every character outside
/// `[a-zA-Z0-9]` a hyphen — so that what lands is what claude itself would have
/// written and nothing here is told where to put it.
const PRINTS_AND_WRITES_ITS_MEMORY: &str = r#"
entry=$(printf '%s' "$PWD" | tr -c 'a-zA-Z0-9' '-')

for file in "$HOME"/.claude/projects/*/*; do
  [ -f "$file" ] || continue
  printf 'memory: %s\n' "$(cat "$file")"
done

mkdir -p "$HOME/.claude/projects/$entry"
printf 'a transcript of the session\n' > "$HOME/.claude/projects/$entry/transcript.jsonl"

for dir in "$HOME"/.claude/projects/*; do
  [ -d "$dir" ] || continue
  case "$dir" in
    */"$entry") ;;
    *) printf 'what the session learned\n' > "$dir/learned.jsonl" ;;
  esac
done

printf 'ran\n'
"#;

/// And what stands there on the device the account is at home on, which starts no
/// session of its own: a Verkstead that keeps what it makes in a Data Directory,
/// because a memory sync asks the home device for its worktrees directory and a
/// router with nowhere to keep anything has none.
const NOTHING_RUNS: &str = "exit 0";

/// A `gh` that answers nothing: no finish step in this suite reaches GitHub, and
/// the real one would need an account.
const NO_GITHUB: &str = "exit 1";

/// How long one address has to answer here, rather than the two seconds a running
/// server gives one.
const PATIENCE: Duration = Duration::from_millis(300);

/// How long a test waits for something two hops away to have happened: a Nudge
/// crossing a link, a list read back over one, a session starting in a real
/// sandbox and printing.
const WAITING: Duration = Duration::from_secs(30);

/// And how often it looks while it waits.
const LOOKING: Duration = Duration::from_millis(50);

/// One Verkstead: its store, its identity, its Peer Listener up behind the Member
/// Gate, and the router its own browser presses.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    nudges: Nudges,
    cluster: Devices,
    workbench: Router,
    address: SocketAddr,

    /// The Data Directory: the identity, the database, the mirrored accounts and
    /// every session's own profile are in it.
    dir: tempfile::TempDir,

    /// What `~` is for a session on this device, which on Linux is the home the
    /// sandbox makes empty over.
    home: tempfile::TempDir,

    /// Held for the length of the test: the accounts the Profiles here name live
    /// in it, away from the Data Directory so that nothing confuses an account
    /// with a mirror of one.
    elsewhere: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead that runs no session: what the device an account is at home on
    /// has to be for this suite, which is a device that answers.
    async fn answering(id: &str) -> Verkstead {
        Verkstead::standing(id, None).await
    }

    /// And one that runs its sessions on `stub`: the device the work happens on.
    async fn running(id: &str, stub: &str) -> Verkstead {
        Verkstead::standing(id, Some(stub)).await
    }

    /// Both of them, which differ in one thing: whether there is an agent to
    /// launch.
    async fn standing(id: &str, stub: Option<&str>) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();

        // Who a session commits as, which every sandbox is configured out of.
        std::fs::write(
            dir.path().join("config.yaml"),
            "git_author:\n  name: Verkstead Test\n  email: test@verkstead.invalid\n",
        )
        .unwrap();

        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();
        let members = Members::recorded(pool.clone());
        let nudges = Nudges::new();

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");
        let address = listener.address();

        let reading = Reading::advertising(
            no_tailscale(),
            Platform::Linux,
            None,
            vec![format!("127.0.0.1:{}", address.port())],
        );

        let cluster = Devices::of(
            device.clone(),
            reading.clone(),
            members.clone(),
            Joins::none(),
        )
        .waiting(PATIENCE);

        let Routers {
            workbench,
            over_the_link,
        } = match stub {
            None => routers_answering_devices_telling(
                pool.clone(),
                cluster.clone(),
                nudges.clone(),
                probing(dir.path()),
            ),

            Some(stub) => routers_running_sessions_answering_devices(
                pool.clone(),
                dir.path().to_owned(),
                agents(stub, home.path(), dir.path()),
                gh_stub(NO_GITHUB),
                cluster.clone(),
                nudges.clone(),
                probing(dir.path()),
            ),
        };

        tokio::spawn(listener.serving(peer::router(
            device.clone(),
            reading,
            members,
            Joins::none(),
            nudges.clone(),
            over_the_link,
        )));

        Verkstead {
            device,
            pool,
            nudges,
            cluster,
            workbench,
            address,
            dir,
            home,
            elsewhere,
        }
    }

    /// Hold a Nudge stream to every member of this device's cluster, which is
    /// what a running server spawns at the start and what refreshes the mirrors.
    fn holding(&self) -> Holding {
        let cluster = self.cluster.clone();
        let nudges = self.nudges.clone();

        Holding(tokio::spawn(async move {
            cluster.stay_fresh(nudges).await;
        }))
    }

    /// The one address it advertises.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// Write `other` down as one of this device's members, at `addresses`.
    async fn linked_to(&self, other: &Device, name: &str, os: &str, addresses: Vec<String>) {
        record_member(
            &self.pool,
            &Linking {
                device: other.id().to_owned(),
                name: name.to_owned(),
                os: os.to_owned(),
                addresses,
                fingerprint: other.fingerprint().to_owned(),
            },
        )
        .await
        .unwrap();
    }

    /// An account of this device's own, really on disk, with a Profile saved over
    /// it the way the form saves one.
    ///
    /// **And the whole of an account around it**, not just the three files a root
    /// is made of: the plugins the human installed, the hooks and the plugin list
    /// in their settings, the MCP servers and the per-repository history in their
    /// `.claude.json`, and another repository's transcripts under `projects/`.
    /// What none of that does is travel — see
    /// [`nothing_lands_in_the_mirror_but_the_files_a_root_is_made_of`].
    async fn account(&self, name: &str) -> i64 {
        let under = self.elsewhere.path().join(name);
        let claude_dir = under.join(".claude");
        let config_file = under.join(".claude.json");

        std::fs::create_dir_all(claude_dir.join("plugins/repos/someone")).unwrap();
        std::fs::write(claude_dir.join("plugins/repos/someone/thing.js"), "//\n").unwrap();
        std::fs::create_dir_all(claude_dir.join("projects/-home-you-src-secrets")).unwrap();
        std::fs::write(
            claude_dir.join("projects/-home-you-src-secrets/one.jsonl"),
            "{\"type\":\"user\"}\n",
        )
        .unwrap();
        std::fs::write(claude_dir.join("CLAUDE.md"), "How I work.\n").unwrap();

        self.logged_in(name, TOKEN);

        std::fs::write(
            claude_dir.join("settings.json"),
            serde_json::json!({
                "env": { "ANTHROPIC_BASE_URL": "https://models.example" },
                "hooks": { "PreToolUse": [{ "command": "the-humans-own-hook" }] },
                "enabledPlugins": ["someone/thing"],
            })
            .to_string(),
        )
        .unwrap();

        std::fs::write(
            &config_file,
            serde_json::json!({
                "oauthAccount": { "emailAddress": "you@example.com" },
                "mcpServers": { "theirs": { "command": "the-humans-own-server" } },
                "projects": {
                    "/home/you/src/secrets": {
                        "hasTrustDialogAccepted": true,
                        "history": [{ "display": "what-they-asked-last" }],
                    },
                },
            })
            .to_string(),
        )
        .unwrap();

        let said = press(
            &self.workbench,
            PROFILES,
            Some(
                &serde_json::json!({
                    "name": name,
                    "account": {
                        "agent_type": "Claude",
                        "claude_dir": claude_dir,
                        "config_file": config_file,
                    },
                    "models": [MODEL],
                    "memory": true,
                })
                .to_string(),
            ),
        )
        .await;

        assert_eq!(said, "\"Saved\"", "saving {name}");

        called(&self.profiles().await, name)
            .expect("the Profile that was just saved is on the list")
            .id
    }

    /// And the same account with its **memory switch off**, which is a session
    /// given a store of its own rather than the account's — at home and away
    /// alike.
    async fn account_forgetting(&self, name: &str) -> i64 {
        let profile = self.account(name).await;

        let said = press(
            &self.workbench,
            &format!("{PROFILES}/{profile}"),
            Some(
                &serde_json::json!({
                    "name": name,
                    "account": {
                        "agent_type": "Claude",
                        "claude_dir": self.elsewhere.path().join(name).join(".claude"),
                        "config_file": self.elsewhere.path().join(name).join(".claude.json"),
                    },
                    "models": [MODEL],
                    "memory": false,
                })
                .to_string(),
            ),
        )
        .await;

        assert_eq!(said, "\"Saved\"", "switching {name}'s memory off");

        profile
    }

    /// A repository of this device's own, registered — which is what makes it one
    /// of this device's Repos for the match across devices to find, and so what
    /// says where that account's memory of it is kept.
    async fn repo(&self) -> PathBuf {
        let path = self.elsewhere.path().join("verkstead");

        if !path.exists() {
            repository(path.clone());
        }

        let registered = press(
            &self.workbench,
            "/api/ui/repos",
            Some(&serde_json::json!({ "path": path }).to_string()),
        )
        .await;

        assert!(
            registered.contains("Added") || registered.contains("AlreadyRegistered"),
            "registering the repository: {registered}",
        );

        path
    }

    /// What that account remembers of the directory at `path`, written into the
    /// entry this harness keeps it under.
    fn remembers(&self, name: &str, path: &Path, file: &str, text: &str) {
        let entry = self.entry(name, path);

        std::fs::create_dir_all(&entry).unwrap();
        std::fs::write(entry.join(file), text).unwrap();
    }

    /// And what is in that entry now, polled until it holds `file` — or a panic
    /// saying what it holds instead.
    ///
    /// Polled because a write-back is the last thing a session's ending does,
    /// after the process has been reaped and the profile seen to.
    async fn remembered(&self, name: &str, path: &Path, file: &str) -> String {
        let entry = self.entry(name, path);
        let deadline = Instant::now() + WAITING;

        loop {
            if let Ok(text) = std::fs::read_to_string(entry.join(file)) {
                return text;
            }

            assert!(
                Instant::now() < deadline,
                "{} never came to hold {file}. It holds: {:?}",
                entry.display(),
                walked(&entry),
            );

            tokio::time::sleep(LOOKING).await;
        }
    }

    /// Where that account keeps what it remembers of the directory at `path`.
    fn entry(&self, name: &str, path: &Path) -> PathBuf {
        self.elsewhere
            .path()
            .join(name)
            .join(".claude/projects")
            .join(entry_named(path))
    }

    /// And the path this device would have cut this Conversation's work in: its
    /// own worktrees directory under its Data Directory, with the stem the other
    /// machine's Worktree carries.
    fn would_have_cut(&self, stem: &str) -> PathBuf {
        self.dir.path().join("worktrees").join(stem)
    }

    /// What the one Worktree this device has cut is called, which is the stem
    /// both machines name theirs with.
    fn worktree_stem(&self) -> String {
        let mut under = walked_directories(&self.dir.path().join("worktrees"));

        assert_eq!(under.len(), 1, "one Worktree has been cut: {under:?}");

        under.pop().expect("one Worktree")
    }

    /// That account signed in, or signed in again with another token — which is
    /// the whole of what a refresh at home does to the one file a session
    /// changes.
    fn logged_in(&self, name: &str, token: &str) {
        std::fs::write(
            self.login_of(name),
            serde_json::json!({ "claudeAiOauth": { "accessToken": token } }).to_string(),
        )
        .unwrap();
    }

    /// And signed out, which is the file going.
    fn signed_out(&self, name: &str) {
        std::fs::remove_file(self.login_of(name)).unwrap();
    }

    /// Where that account keeps its login.
    fn login_of(&self, name: &str) -> PathBuf {
        self.elsewhere
            .path()
            .join(name)
            .join(".claude/.credentials.json")
    }

    /// And what is in it, which on the home device is what a write-back landed.
    fn login_at_home(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.login_of(name)).ok()
    }

    /// The same, polled until it says `saying` — or a panic saying what it holds
    /// instead.
    ///
    /// Polled because a write-back is the last thing a session's ending does, after
    /// the process has been reaped and the profile seen to: what is being waited for
    /// is a real session finishing and a real call crossing a real link.
    async fn logged_in_with(&self, name: &str, saying: &str) -> String {
        let deadline = Instant::now() + WAITING;

        loop {
            let held = self.login_at_home(name).unwrap_or_default();

            if held.contains(saying) {
                return held;
            }

            assert!(
                Instant::now() < deadline,
                "the account {name} never came to hold {saying:?}. It holds: {held:?}",
            );

            tokio::time::sleep(LOOKING).await;
        }
    }

    /// The Profiles as this device's own browser reads them.
    async fn profiles(&self) -> Vec<ProfileEntry> {
        reading(&self.workbench, PROFILES).await
    }

    /// The mirror of the member's Profile called `name`, once the refresher has
    /// written it down.
    async fn mirror_of(&self, name: &str) -> ProfileEntry {
        let mut last = Vec::new();

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                last = self.profiles().await;

                if let Some(row) = called(&last, name) {
                    return row.clone();
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("no mirror of {name} was ever written down: {last:#?}"))
    }

    /// The account of that row fetched from the device it is at home on, exactly
    /// as a launch here fetches it — the Profile the launch then reads, naming this
    /// device's own mirror as its account, and what its ending has to write home.
    async fn fetches(&self, row: &ProfileEntry) -> Result<mirroring::account::Mirrored, Refusal> {
        let profile = store::load_profile(&self.pool, row.id)
            .await
            .unwrap()
            .expect("the mirror row is in this device's store");

        mirroring::account::fetched(Some(&self.cluster), &self.homes(), &profile)
            .await
            .map(|fetched| fetched.expect("a mirror is a row whose account is fetched"))
    }

    /// And that session ending, which is where the login it left goes home —
    /// exactly as a session's relay and a terminal's follow loop see to it, once
    /// everything they were given has been written back into the mirror.
    ///
    /// `conversation` is whose Timeline a home that has gone away is said on.
    async fn ends(&self, lent: mirroring::account::Lent, conversation: i64) {
        mirroring::account::Lending::of(&self.pool, Some(&self.cluster), lent)
            .written_home(conversation)
            .await;
    }

    /// A Conversation of this device's own to say something on, with nothing
    /// running in it: what a Timeline sentence about an ending needs is a
    /// Conversation to be on.
    async fn conversation(&self) -> i64 {
        let path = repository(self.elsewhere.path().join("verkstead"));

        let registered = press(
            &self.workbench,
            "/api/ui/repos",
            Some(&serde_json::json!({ "path": path }).to_string()),
        )
        .await;

        assert!(registered.contains("Added"), "registering the repository");

        let repos: Vec<verkstead_render::RepoEntry> =
            reading(&self.workbench, "/api/ui/repos").await;

        let started = press(
            &self.workbench,
            "/api/ui/conversations",
            Some(&serde_json::json!({ "repo_id": repos[0].id }).to_string()),
        )
        .await;

        let Started::Started { id, .. } = serde_json::from_str(&started).unwrap() else {
            panic!("the Conversation was not started: {started}")
        };

        id
    }

    /// Every Notice on that Conversation's Timeline, which is where Verkstead says
    /// what it did on its own account.
    async fn notices(&self, conversation: i64) -> Vec<String> {
        let drawn: ConversationView = reading(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}"),
        )
        .await;

        drawn
            .timeline
            .iter()
            .filter_map(|event| match event {
                TimelineEvent::Notice(notice) => Some(notice.html.clone()),
                _ => None,
            })
            .collect()
    }

    /// The homes a session on this device is given, which is what says where a
    /// mirror goes: under this Data Directory.
    fn homes(&self) -> Homes {
        Homes::on(Platform::HERE, self.home.path().to_owned(), self.dir.path())
    }

    /// Where this device keeps its mirror of the Profile with that local id.
    fn mirror_directory(&self, profile: i64) -> PathBuf {
        self.dir.path().join("accounts").join(profile.to_string())
    }

    /// A Conversation grilling under `profile`, started the way the composer and
    /// the setup card start one: a real repository, all three Pairings on that
    /// Profile, a Brief, and the press.
    async fn grilling_under(&self, profile: i64) -> i64 {
        let path = repository(self.elsewhere.path().join("verkstead"));

        let registered = press(
            &self.workbench,
            "/api/ui/repos",
            Some(&serde_json::json!({ "path": path }).to_string()),
        )
        .await;

        assert!(registered.contains("Added"), "registering the repository");

        let repos: Vec<verkstead_render::RepoEntry> =
            reading(&self.workbench, "/api/ui/repos").await;

        let started = press(
            &self.workbench,
            "/api/ui/conversations",
            Some(&serde_json::json!({ "repo_id": repos[0].id }).to_string()),
        )
        .await;

        let Started::Started {
            id: conversation, ..
        } = serde_json::from_str(&started).unwrap()
        else {
            panic!("the Conversation was not started: {started}")
        };

        let pairing = serde_json::json!({ "profile_id": profile, "model": MODEL });
        let role = serde_json::json!({ "pairing": pairing });

        for (path, saying) in [
            ("grilling-pairing", &pairing),
            ("implementation-pairing", &pairing),
            ("review-pairing", &role),
        ] {
            let said = press(
                &self.workbench,
                &format!("/api/ui/conversations/{conversation}/{path}"),
                Some(&saying.to_string()),
            )
            .await;

            assert_eq!(said, "\"Chosen\"", "picking {path}");
        }

        let saved = press(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/brief"),
            Some(&serde_json::json!({ "markdown": BRIEF }).to_string()),
        )
        .await;

        assert_eq!(saved, "\"Saved\"", "writing the Brief");

        let grilling = press(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/grill"),
            None,
        )
        .await;

        assert_eq!(grilling, "\"Started\"", "starting the grilling");

        conversation
    }

    /// What the session on that Conversation printed, once it has printed
    /// `saying` — or a panic saying what it did print instead.
    ///
    /// Polled rather than waited on a signal: what is being waited for is a real
    /// process starting in a real sandbox, which is the slow part of a launch.
    async fn printed(&self, conversation: i64, saying: &str) -> String {
        let deadline = Instant::now() + WAITING;
        let mut said = String::new();

        loop {
            said = match self.capture(conversation).await {
                Some(capture) => capture,
                None => said,
            };

            if said.contains(saying) {
                return said;
            }

            assert!(
                Instant::now() < deadline,
                "the session never printed {saying:?}. It printed: {said:?}",
            );

            tokio::time::sleep(LOOKING).await;
        }
    }

    /// The whole of what the session on that Conversation has said, as the
    /// details pane fetches it.
    async fn capture(&self, conversation: i64) -> Option<String> {
        let drawn: ConversationView = reading(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}"),
        )
        .await;

        let event = drawn.timeline.iter().find_map(|event| match event {
            TimelineEvent::AgentOutput(output) => Some(output.id),
            _ => None,
        })?;

        let capture: Capture = reading(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/capture/{event}"),
        )
        .await;

        Some(capture.text)
    }
}

/// The agent a session on this device runs, and everything else a launch is
/// equipped out of.
fn agents(stub: &str, home: &Path, data_dir: &Path) -> Agents {
    Agents::running(
        vec!["/bin/sh".to_owned(), "-c".to_owned(), stub.to_owned()],
        Homes::on(Platform::HERE, home.to_owned(), data_dir),
        Reachable::at(LISTENING),
        SandboxConfig::default(),
        // Nothing here builds anything: what runs where claude goes is a shell
        // script.
        BuildCache::none(),
        Skills::installed(Platform::HERE, data_dir).expect("this binary carries skills"),
        Executable::of_the_server(data_dir),
        Handoffs::under(data_dir),
        Attachments::under(data_dir),
        Settings::in_data_dir(data_dir),
    )
}

/// Where the server these sessions belong to would be listening. Nothing dials
/// it: a router driven by `oneshot` has no socket, and what this is for is the
/// `VERKSTEAD_SERVER` a session inside is told.
const LISTENING: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 8422);

/// A `gh` that is a shell script, so nothing here reaches GitHub.
fn gh_stub(script: &str) -> Gh {
    Gh::running(vec![
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        script.to_owned(),
        "gh".to_owned(),
    ])
}

/// The streams one device is holding, given back when the test drops it.
struct Holding(tokio::task::JoinHandle<()>);

impl Drop for Holding {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422)
}

/// The machine a device here is judged on: one directory on the `PATH` a
/// session would search, holding a program for every harness.
///
/// **Stated rather than read off the runner.** A **mirror** whose harness is
/// not on this device reads broken and starts nothing, which is exactly right
/// and exactly not what this suite is about: what runs a session here is the
/// stub the agents handle names, and whether the box this runs on happens to
/// have `claude` installed is nothing to do with it. So every device says it
/// has all four. The suite that asks what a device *without* one does is
/// `tests/mirroring.rs`.
fn probing(dir: &Path) -> Machine {
    let bin = dir.join("probed");
    std::fs::create_dir_all(&bin).unwrap();

    for harness in ["claude", "codex", "grok", "opencode"] {
        let at = bin.join(harness);

        std::fs::write(&at, "#!/bin/sh\nexit 0\n").unwrap();
        std::fs::set_permissions(&at, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    Machine::stated(
        Platform::Linux,
        bin.as_os_str().to_owned(),
        bin.as_os_str().to_owned(),
        None,
        None,
        &Environment {
            home: Some(dir.to_owned()),
            ..Environment::default()
        },
    )
}

/// A, holding the account, and B, running the session — linked both ways, with B
/// holding the streams that refresh its mirrors.
async fn linked_up() -> (Verkstead, Verkstead, Holding) {
    linked_up_running(PRINTS_ITS_ROOT).await
}

/// The same, with the session B runs said: what a session does to the account it
/// was lent is the other half of this suite, and what it does is what the stub
/// does.
async fn linked_up_running(stub: &str) -> (Verkstead, Verkstead, Holding) {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::running(B, stub).await;

    joined(&a, &b).await;

    let holding = b.holding();

    (a, b, holding)
}

/// The same with **both** devices keeping what they make in a Data Directory,
/// which is what the memory sync needs of the device at home: the entry a
/// Worktree's memory is kept under there is named off that machine's own
/// worktrees directory, and a router given nowhere to keep anything has none.
async fn linked_up_at_home_too(stub: &str) -> (Verkstead, Verkstead, Holding) {
    let a = Verkstead::running(A, NOTHING_RUNS).await;
    let b = Verkstead::running(B, stub).await;

    joined(&a, &b).await;

    let holding = b.holding();

    (a, b, holding)
}

/// `b` and `a` linked both ways, which is what a member is: each device holds the
/// other's certificate and the address to reach it at.
async fn joined(a: &Verkstead, b: &Verkstead) {
    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;

    let (machine, os) = this_machine();
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;
}

/// What stands where claude goes when the question is the **write-back**: a
/// session that refreshes its own login and stops, which is the one thing a
/// harness does to an account it is running as.
///
/// **Written in place rather than renamed over**, which is how a harness that keeps
/// its login in a file saves one — and what a bind takes: on this platform the
/// login in the root is a bind of the mirror's own file.
fn refreshes_its_login(token: &str) -> String {
    format!(
        "printf '{{\"claudeAiOauth\":{{\"accessToken\":\"%s\"}}}}' '{token}' \
> \"$HOME/.claude/.credentials.json\"\nprintf 'refreshed the login\\n'\n"
    )
}

/// What a device says about the machine it is on, read the way the server reads
/// it.
fn this_machine() -> (String, String) {
    (
        platform::hostname(),
        platform::os_word(Platform::Linux, None),
    )
}

/// A git repository at `path`, with one commit on `main` so there is a branch to
/// cut a worktree off.
fn repository(path: PathBuf) -> PathBuf {
    std::fs::create_dir_all(&path).unwrap();
    git(&path, &["init", "--initial-branch", "main"]);
    git(&path, &["config", "user.email", "test@verkstead.invalid"]);
    git(&path, &["config", "user.name", "Verkstead Test"]);
    std::fs::write(path.join("README.md"), "# a repository\n").unwrap();
    git(&path, &["add", "README.md"]);
    git(&path, &["commit", "-m", "first"]);

    path
}

/// Run git in `dir`, and fail the test where it does not.
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("git should be on the PATH for these tests");

    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// The name Claude gives the `projects/` entry for `path`, worked out here
/// rather than asked of the server: what the memory sync has to get right is
/// that both machines name an entry the harness's own way, and a test that asked
/// the code under test what the name was would be asking it to agree with
/// itself.
///
/// Every UTF-16 unit outside `[a-zA-Z0-9]` becomes `-`, one for one. Nothing here
/// is long enough for the cut and the hash that follow it.
fn entry_named(path: &Path) -> String {
    path.to_string_lossy()
        .encode_utf16()
        .map(|unit| match char::from_u32(u32::from(unit)) {
            Some(kept) if kept.is_ascii_alphanumeric() => kept,
            _ => '-',
        })
        .collect()
}

/// The directories directly under `dir`, by name and in order.
fn walked_directories(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();

    found.sort();
    found
}

/// One row of a Profiles list, by the name on it.
fn called<'a>(rows: &'a [ProfileEntry], name: &str) -> Option<&'a ProfileEntry> {
    rows.iter().find(|row| row.name.as_deref() == Some(name))
}

/// A read of whatever `path` answers, made the way the browser makes one.
async fn reading<T: serde::de::DeserializeOwned>(app: &Router, path: &str) -> T {
    let answered = app
        .clone()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let status = answered.status();
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    let said = String::from_utf8_lossy(&bytes).into_owned();

    assert_eq!(status, StatusCode::OK, "GET {path}: {said}");

    serde_json::from_str(&said).unwrap_or_else(|why| panic!("GET {path} answered {said}: {why}"))
}

/// And a press on `path`, with a body where the endpoint takes one.
async fn press(app: &Router, path: &str, saying: Option<&str>) -> String {
    let asking = Request::builder().method("POST").uri(path);

    let asking = match saying {
        Some(body) => asking
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_owned())),
        None => asking.body(Body::empty()),
    };

    let answered = app.clone().oneshot(asking.unwrap()).await.unwrap();

    let status = answered.status();
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    let said = String::from_utf8_lossy(&bytes).into_owned();

    assert!(status.is_success(), "POST {path}: {status} {said}");

    said
}

/// What the account's login file holds, read off the mirror the way a session's
/// root reads it.
fn login_in(mirror: &Path) -> Option<String> {
    std::fs::read_to_string(mirror.join(".claude/.credentials.json")).ok()
}

/// Every file under `dir`, said from `dir` and in order, for the assertion that
/// nothing else landed there.
fn walked(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut looking = vec![dir.to_owned()];

    while let Some(at) = looking.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                looking.push(path);
            } else {
                found.push(
                    path.strip_prefix(dir)
                        .expect("everything walked is under the directory")
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }

    found.sort();
    found
}

/// A session on B under a Profile whose account is on A **runs, and is logged
/// in**: the login A holds is in the root the session was given, and the
/// configuration beside it is the allowlist's worth of A's and no more.
#[tokio::test]
async fn a_session_away_from_home_runs_under_the_account_at_home() {
    let (a, b, _holding) = linked_up().await;

    a.account("work").await;

    let mirror = b.mirror_of("work").await;

    assert_eq!(
        mirror.broken, None,
        "a member's account is a row to run under: {mirror:?}",
    );

    let conversation = b.grilling_under(mirror.id).await;
    let said = b.printed(conversation, "config: ").await;

    assert!(
        said.contains(TOKEN),
        "the session is logged in as the account on {A_MACHINE}. It printed: {said:?}",
    );
    assert!(
        said.contains("https://models.example"),
        "and reaches the model the way that account does. It printed: {said:?}",
    );
    assert!(
        said.contains("you@example.com"),
        "and its `.claude.json` says whose account it is. It printed: {said:?}",
    );

    // And none of how the human on A works, which is what a root never carries
    // and therefore what a mirror never holds.
    for theirs in [
        "the-humans-own-hook",
        "someone/thing",
        "the-humans-own-server",
        "what-they-asked-last",
    ] {
        assert!(
            !said.contains(theirs),
            "{theirs} is how the human on {A_MACHINE} works and is no part of a \
             session's root. It printed: {said:?}",
        );
    }

    // And what it ran out of is this device's own mirror of that account, under
    // its Data Directory — which is what makes the login a hard link can join in
    // on Windows and what a session end has to write back.
    assert_eq!(
        login_in(&b.mirror_directory(mirror.id))
            .as_deref()
            .map(|login| login.contains(TOKEN)),
        Some(true),
        "the account the session ran as is the mirror under B's Data Directory",
    );
}

/// A login refreshed at home between two launches is the one the second launch
/// gets — and a sign-out at home takes the login off the mirror rather than
/// leaving the last one it saw standing.
#[tokio::test]
async fn a_login_refreshed_at_home_is_the_one_the_next_launch_gets() {
    let (a, b, _holding) = linked_up().await;

    a.account("work").await;

    let mirror = b.mirror_of("work").await;
    let under = b.mirror_directory(mirror.id);

    let fetched = b.fetches(&mirror).await.expect("A answered");

    assert_eq!(
        fetched.profile.account,
        verkstead_store::Account::Claude {
            claude_dir: under.join(".claude"),
            config_file: under.join(".claude.json"),
        },
        "the launch reads the mirror as the account, in the shape the harness \
         keeps one",
    );
    assert!(
        login_in(&under).is_some_and(|login| login.contains(TOKEN)),
        "the login A holds",
    );

    a.logged_in("work", REFRESHED);
    b.fetches(&mirror).await.expect("A answered again");

    let login = login_in(&under).expect("the mirror holds a login");

    assert!(
        login.contains(REFRESHED) && !login.contains(TOKEN),
        "and the one it holds now, fetched again before this launch: {login:?}",
    );

    a.signed_out("work");
    b.fetches(&mirror).await.expect("A answered once more");

    assert_eq!(
        login_in(&under),
        None,
        "and a sign-out at home is a mirror with no login in it rather than one \
         holding a login nobody can use",
    );
}

/// Nothing lands in the mirror but the files a Built Root is made of, checked
/// against an account with plugins, hooks and another repository's transcripts in
/// it.
#[tokio::test]
async fn nothing_lands_in_the_mirror_but_the_files_a_root_is_made_of() {
    let (a, b, _holding) = linked_up().await;

    a.account("work").await;

    let mirror = b.mirror_of("work").await;
    let under = b.mirror_directory(mirror.id);

    b.fetches(&mirror).await.expect("A answered");

    assert_eq!(
        walked(&under),
        vec![
            ".claude.json".to_owned(),
            ".claude/.credentials.json".to_owned(),
            ".claude/settings.json".to_owned(),
        ],
        "the login and what the written configuration is composed from, and \
         nothing else of an account that holds plenty else",
    );
}

/// A home that is not answering is a launch refused **by name**: the account was
/// never fetched, and a session started anyway would come up logged out.
#[tokio::test]
async fn a_home_that_is_not_answering_refuses_the_launch_by_name() {
    let b = Verkstead::running(B, PRINTS_ITS_ROOT).await;

    // A device that is a member and is not there: the identity is real, so the
    // membership row is a real one, and nothing is listening at the address.
    let away = tempfile::tempdir().unwrap();
    let asleep = Device::stated(away.path(), A).unwrap();

    b.linked_to(&asleep, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    // What it last gave, which on a running server is what the last refresh
    // wrote down.
    store::record_mirror(
        &b.pool,
        &verkstead_store::Mirror {
            device: A.to_owned(),
            id: 7,
            login: true,
        },
        &store::ProfileFacts {
            name: Some("work".to_owned()),
            account: store::Account::Claude {
                claude_dir: PathBuf::from("/home/you/accounts/work/.claude"),
                config_file: PathBuf::from("/home/you/accounts/work/.claude.json"),
            },
            models: vec![MODEL.to_owned()],
            memory: true,
        },
    )
    .await
    .unwrap();

    let mirror = b.mirror_of("work").await;

    let why = b
        .fetches(&mirror)
        .await
        .expect_err("the machine the account is on is not answering");

    assert_eq!(why.status, StatusCode::BAD_GATEWAY);
    assert!(
        why.saying.contains(A_MACHINE),
        "the sentence names the machine the human named: {}",
        why.saying,
    );

    assert_eq!(
        login_in(&b.mirror_directory(mirror.id)),
        None,
        "and nothing was written: a mirror half-fetched would be a session \
         running as somebody nothing said was signed in",
    );
}

/// A token refreshed **inside a session on B** is in A's account once that session
/// ends: the login is the one file of a root a session genuinely changes, and an
/// account lent out and never written back would be one signing itself out a
/// session at a time.
#[tokio::test]
async fn a_token_refreshed_in_a_session_away_from_home_lands_in_the_account() {
    const INSIDE: &str = "sk-ant-oat01-refreshed-in-the-session";

    let (a, b, _holding) = linked_up_running(&refreshes_its_login(INSIDE)).await;

    a.account("work").await;

    let mirror = b.mirror_of("work").await;
    let conversation = b.grilling_under(mirror.id).await;

    b.printed(conversation, "refreshed the login").await;

    let held = a.logged_in_with("work", INSIDE).await;

    assert!(
        !held.contains(TOKEN),
        "the account holds what the session left rather than what it was lent: \
         {held:?}",
    );
    assert_eq!(
        login_in(&b.mirror_directory(mirror.id)).as_deref(),
        Some(held.as_str()),
        "and the mirror is left where it is: the account was written, and \
         nothing here was taken away",
    );
}

/// And only what changed travels: a login the session replaced goes back whole,
/// one it left exactly as it was given is not written at all, and one made where
/// the account had none is handed over.
///
/// **The mirror's own login is written here rather than by a session**, which is
/// what a session's ending has already done by the time the write-back runs — see
/// `a_token_refreshed_in_a_session_away_from_home_lands_in_the_account` for the
/// whole of that path. What is being asked is which of the three cases sends
/// anything.
#[tokio::test]
async fn only_a_login_the_session_changed_goes_back_home() {
    let (a, b, _holding) = linked_up().await;

    a.account("work").await;

    let mirror = b.mirror_of("work").await;
    let conversation = b.conversation().await;
    let under = b.mirror_directory(mirror.id);

    // Untouched: the ending writes nothing, which is read off a home that has
    // refreshed its own login since the fetch — a write of what came down would
    // have put the old token back over it.
    let lent = b.fetches(&mirror).await.expect("A answered").lent;

    a.logged_in("work", REFRESHED);
    b.ends(lent, conversation).await;

    let held = a.login_at_home("work").expect("the account holds a login");

    assert!(
        held.contains(REFRESHED) && !held.contains(TOKEN),
        "a login the session left alone is not written, so the refresh at home \
         stands: {held:?}",
    );

    // Replaced: what the session left goes back whole.
    let lent = b.fetches(&mirror).await.expect("A answered again").lent;
    let refreshed = r#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-and-away"}}"#;

    std::fs::write(under.join(".claude/.credentials.json"), refreshed).unwrap();
    b.ends(lent, conversation).await;

    assert_eq!(
        a.login_at_home("work").as_deref(),
        Some(refreshed),
        "a login the session replaced goes back whole",
    );

    // And one made where the account had none, which is the case a Profile
    // nobody has logged in to starts in.
    a.signed_out("work");

    let lent = b.fetches(&mirror).await.expect("A answered once more").lent;

    assert_eq!(login_in(&under), None, "nothing came down in the login");

    let made = r#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-logged-in-away"}}"#;

    std::fs::write(under.join(".claude/.credentials.json"), made).unwrap();
    b.ends(lent, conversation).await;

    assert_eq!(
        a.login_at_home("work").as_deref(),
        Some(made),
        "and a login made where the account had none is handed over",
    );
}

/// Two devices refreshing the one login end on the **later write**: nothing is
/// locked and nothing is merged, and what the account keeps is whatever arrived
/// last.
#[tokio::test]
async fn two_devices_refreshing_one_login_end_on_the_later_write() {
    /// The third device, which is the second one away from home.
    const C: &str = "ffeeddccbbaa00998877665544332211";

    let a = Verkstead::answering(A).await;
    let b = Verkstead::running(B, PRINTS_ITS_ROOT).await;
    let c = Verkstead::running(C, PRINTS_ITS_ROOT).await;

    joined(&a, &b).await;
    joined(&a, &c).await;

    let (_b_holding, _c_holding) = (b.holding(), c.holding());

    a.account("work").await;

    // Both fetch the account, which is two sessions running at once under the one
    // Profile — neither of them holding anything against the other.
    let (b_mirror, c_mirror) = (b.mirror_of("work").await, c.mirror_of("work").await);

    let b_lent = b.fetches(&b_mirror).await.expect("A answered B").lent;
    let c_lent = c.fetches(&c_mirror).await.expect("A answered C").lent;

    let (b_refreshed, c_refreshed) = (
        r#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-b-refreshed"}}"#,
        r#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-c-refreshed"}}"#,
    );

    std::fs::write(
        b.mirror_directory(b_mirror.id)
            .join(".claude/.credentials.json"),
        b_refreshed,
    )
    .unwrap();
    std::fs::write(
        c.mirror_directory(c_mirror.id)
            .join(".claude/.credentials.json"),
        c_refreshed,
    )
    .unwrap();

    let (b_conversation, c_conversation) = (b.conversation().await, c.conversation().await);

    b.ends(b_lent, b_conversation).await;

    assert_eq!(
        a.login_at_home("work").as_deref(),
        Some(b_refreshed),
        "the first to end is the login the account holds meanwhile",
    );

    c.ends(c_lent, c_conversation).await;

    assert_eq!(
        a.login_at_home("work").as_deref(),
        Some(c_refreshed),
        "and the write that arrived later is the one the account keeps — nothing \
         merged, and nothing held against the second device",
    );
}

/// A home that has gone away by the time the session ends leaves the mirror where
/// it is and puts a **sentence on the Timeline** saying the account was not
/// written: the next session at home may find itself signed out, and that is worth
/// a line rather than a silent loss.
#[tokio::test]
async fn a_home_that_has_gone_away_is_said_on_the_timeline() {
    let (a, b, _holding) = linked_up().await;

    a.account("work").await;

    let mirror = b.mirror_of("work").await;
    let conversation = b.conversation().await;
    let under = b.mirror_directory(mirror.id);

    let lent = b.fetches(&mirror).await.expect("A answered").lent;

    // And then the machine moves: what B has written down for A is an address
    // nothing is listening at, which is what a device switched off comes to.
    b.linked_to(&a.device, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    let refreshed = r#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-nowhere-to-go"}}"#;

    std::fs::write(under.join(".claude/.credentials.json"), refreshed).unwrap();
    b.ends(lent, conversation).await;

    let notices = b.notices(conversation).await;
    let said = notices
        .iter()
        .find(|notice| notice.contains("login"))
        .unwrap_or_else(|| {
            panic!("nothing on the Timeline says the account was not written: {notices:#?}")
        });

    assert!(
        said.contains(A_MACHINE),
        "the sentence names the machine the human named: {said}",
    );

    assert_eq!(
        login_in(&under).as_deref(),
        Some(refreshed),
        "and the mirror is left exactly where it is",
    );
    assert!(
        a.login_at_home("work")
            .is_some_and(|held| held.contains(TOKEN)),
        "the account at home is what it was, nothing having reached it",
    );
}

/// A session on B under A's Profile **starts with what A's account remembered of
/// this repository**, and a memory it writes is in A's account afterwards.
///
/// Which of B's Repos is A's is git's own answer on both ends, and the entry each
/// machine keeps that memory under is named off its own path for the repository:
/// what crosses the link is the files and which part of the store they are in.
#[tokio::test]
async fn a_session_away_from_home_starts_with_what_the_account_remembered() {
    let (a, b, _holding) = linked_up_at_home_too(PRINTS_AND_WRITES_ITS_MEMORY).await;

    let at_home = a.repo().await;

    a.account("work").await;
    a.remembers(
        "work",
        &at_home,
        "remembered.jsonl",
        "what this account remembers of the repository\n",
    );

    let mirror = b.mirror_of("work").await;
    let conversation = b.grilling_under(mirror.id).await;
    let said = b.printed(conversation, "ran").await;

    assert!(
        said.contains("what this account remembers of the repository"),
        "the session started with what A remembered of this repository. It printed: {said:?}",
    );

    assert_eq!(
        a.remembered("work", &at_home, "learned.jsonl").await,
        "what the session learned\n",
        "and what it wrote is in the account on {A_MACHINE}",
    );
}

/// And **its transcript is readable on A** once the session ends, under the entry
/// A's own Worktree for that branch would carry.
///
/// A Worktree lives under the Data Directory rather than under the Repo, so there
/// is no match to ask for: the entry is named off A's own worktrees directory
/// with the same stem B's Worktree carries, which is the path A would have used
/// for this work.
#[tokio::test]
async fn the_transcript_of_a_session_away_from_home_comes_home() {
    let (a, b, _holding) = linked_up_at_home_too(PRINTS_AND_WRITES_ITS_MEMORY).await;

    a.repo().await;
    a.account("work").await;

    let mirror = b.mirror_of("work").await;
    let conversation = b.grilling_under(mirror.id).await;

    b.printed(conversation, "ran").await;

    let stem = b.worktree_stem();

    assert_eq!(
        a.remembered("work", &a.would_have_cut(&stem), "transcript.jsonl")
            .await,
        "a transcript of the session\n",
        "the transcript is under the entry {A_MACHINE}'s own Worktree would carry",
    );
}

/// **No Repo match pulls nothing, says so on the Timeline, and still starts the
/// session** — the Worktree's half of the store travelling either way, being
/// named off the Data Directory rather than off the Repo.
#[tokio::test]
async fn no_repo_match_is_said_on_the_timeline_and_starts_the_session_anyway() {
    let (a, b, _holding) = linked_up_at_home_too(PRINTS_AND_WRITES_ITS_MEMORY).await;

    // Nothing of A's is registered, so nothing of A's is this repository.
    a.account("work").await;

    let mirror = b.mirror_of("work").await;
    let conversation = b.grilling_under(mirror.id).await;

    b.printed(conversation, "ran").await;

    let said = b.notices(conversation).await.join("\n");

    assert!(
        said.contains("no Repo that is this repository"),
        "the Timeline says what was not pulled. It says: {said:?}",
    );

    let stem = b.worktree_stem();

    assert_eq!(
        a.remembered("work", &a.would_have_cut(&stem), "transcript.jsonl")
            .await,
        "a transcript of the session\n",
        "and the session ran, and its transcript came home",
    );
}

/// **Memory switched off syncs nothing in either direction and the session starts
/// empty** — which is what the switch means at home, and means away from home for
/// the same reason.
#[tokio::test]
async fn a_profile_that_shares_no_memory_syncs_nothing_away_from_home() {
    let (a, b, _holding) = linked_up_at_home_too(PRINTS_AND_WRITES_ITS_MEMORY).await;

    let at_home = a.repo().await;

    a.account_forgetting("work").await;
    a.remembers(
        "work",
        &at_home,
        "remembered.jsonl",
        "what this account remembers of the repository\n",
    );

    let mirror = b.mirror_of("work").await;
    let conversation = b.grilling_under(mirror.id).await;
    let said = b.printed(conversation, "ran").await;

    assert!(
        !said.contains("what this account remembers of the repository"),
        "the session started on a store of its own. It printed: {said:?}",
    );

    // And the ending carried nothing back: what A holds is what A held, with
    // nothing of this session's beside it.
    let entry = a.entry("work", &at_home);

    assert_eq!(
        walked(&entry),
        ["remembered.jsonl"],
        "nothing of the session's landed in the account at home",
    );

    assert!(
        !a.would_have_cut(&b.worktree_stem()).exists()
            && walked(&a.entry("work", &a.would_have_cut(&b.worktree_stem()))).is_empty(),
        "and nothing was written under the entry its Worktree would have carried",
    );
}
