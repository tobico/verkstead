//! **The move itself**: a Conversation pressed onto another device of the
//! cluster, and what each end holds afterwards (ADR-0020, *Transfer*).
//!
//! **Two Verksteads, a real dial and a real session.** `tests/preflight.rs` is
//! the reading that says whether the press can be made; this is the press, the
//! wait for the turn to end, and the record crossing the link. So nothing here
//! is stood in for but the agent: the repositories are repositories, the
//! session is a real session in a real sandbox, and the Conversation lands on B
//! through the Peer Listener behind the Member Gate.
//!
//! **What stands where claude goes is a shell script that waits at a gate the
//! test opens**, which is `tests/sessions.rs`'s way of holding a turn open: the
//! press has to arrive while a session is genuinely part way through one, and
//! what the press promises is that nothing is cut short.
//!
//! **What crosses is the Conversation, its whole record and its work** — the
//! row first, carrying the Repo the matching settled, the branch, the lifecycle,
//! the Pairings as B's own Profile ids, the Rank verbatim and the birth key;
//! then the **slice**, which is the Timeline and everything hanging off it,
//! every id renumbered as it lands; then the **checkout**, which is the branch
//! as a bundle and the Worktree's working changes beside it.
//!
//! **And the repositories are two ways round.** The tests about the record hold
//! two repositories made separately, sharing a name and nothing else, which is
//! the match doing its work; the tests about the git leg hold a clone of A's on
//! B, which is what a cluster holding one repository actually looks like — a
//! bundle packed against a history the far end already has is the whole point of
//! that leg. See `Repositories`.
//!
//! **On the machine this suite is about**: the sandbox is bwrap and the
//! terminal is a real pseudo-terminal, which is what `tests/sessions.rs` and
//! `tests/accounts.rs` are written for and the reason this file is Unix-only
//! too.
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
use tokio::task::JoinHandle;
use tower::ServiceExt;
use verkstead_render::{
    ConversationEntry, ConversationView, Lifecycle, ProfileEntry, Started, TimelineEvent,
};
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
    Agents, Gh, Routers, open_database, routers_answering_devices_telling,
    routers_running_sessions_answering_devices, store,
};
use verkstead_store::{Linking, record_member};

/// The device the work is on, where every press is made.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the device it is moved onto.
const B: &str = "0011223344556677889900aabbccddee";

/// What B is written down as on A: the name the human gave the machine, which is
/// what the Timeline says the work is going to rather than sixteen bytes of hex.
const B_MACHINE: &str = "the-laptop";
const B_OS: &str = "macOS 15.1";

/// The Profiles section, which is where an account is saved and where a mirror
/// of a member's turns up.
const PROFILES: &str = "/api/ui/profiles";

/// The sidebar, which is where a **Rank** and a **birth key** are read off a row.
const SIDEBAR: &str = "/api/ui/conversations";

/// The account the work runs under, and the model it lists.
const ACCOUNT: &str = "work";
const MODEL: &str = "claude-opus-5";

/// What every repository in this suite is called. Neither end has an origin, so
/// the match is by name — which is the ordinary case for two clones nobody has
/// pushed anywhere.
const REPOSITORY: &str = "verkstead";

/// And the **Companion Repo** beside it, where a Conversation's work is in more
/// than one repository.
const COMPANION: &str = "askance";

/// The Brief every Conversation here is started from.
const BRIEF: &str = "# Rate limiting\n\nThe API has none.\n";

/// A `gh` that answers nothing: nothing here reaches GitHub, and the real one
/// would need an account.
const NO_GITHUB: &str = "exit 1";

/// How long one address has to answer here, rather than the two seconds a
/// running server gives one.
const PATIENCE: Duration = Duration::from_millis(300);

/// How long a test waits for something two hops away: a session starting in a
/// real sandbox, a mirror written off a member's list, a record crossing a link.
const WAITING: Duration = Duration::from_secs(30);

/// And how often it looks while it waits.
const LOOKING: Duration = Duration::from_millis(50);

/// Where the server these sessions belong to would be listening. Nothing dials
/// it: a router driven by `oneshot` has no socket, and what this is for is the
/// `VERKSTEAD_SERVER` a session inside is told.
const LISTENING: SocketAddr =
    SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 8422);

/// One Verkstead: its store, its identity, its Peer Listener up behind the
/// Member Gate, and the router its own browser presses.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    nudges: Nudges,
    cluster: Devices,
    workbench: Router,
    address: SocketAddr,

    /// The listener, held so that a test can switch this machine off part way
    /// through — which is what a move that fails between the press and the
    /// crossing is made of.
    listening: JoinHandle<anyhow::Result<()>>,

    /// The Data Directory: the identity, the database, every session's own
    /// profile and the attached files are in it. Held for the length of the test
    /// so that none of it is swept out from under a running session — and read
    /// for the one thing a test looks at directly, which is where an Attachment
    /// landed.
    _dir: tempfile::TempDir,

    /// And what `~` is for a session on this device, held for the same reason.
    _home: tempfile::TempDir,

    /// And where this device's repositories and accounts are, away from the Data
    /// Directory so that nothing confuses a repository with a Worktree.
    elsewhere: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead equipped the way a served one is: its sessions run on `stub`,
    /// with `spill` bound into the sandbox so one can see the gate the test
    /// opens.
    ///
    /// **Both machines, though only A ever runs anything.** Nothing starts a
    /// session on the far end in this stage — that is Resume, and it is stage
    /// 07's — but a device taking work in needs what a served one has: a Data
    /// Directory, which is where an arriving Conversation's attached files land.
    async fn running(id: &str, stub: &str, spill: &Path) -> Verkstead {
        Verkstead::standing(id, Some(stub), spill).await
    }

    async fn standing(id: &str, stub: Option<&str>, spill: &Path) -> Verkstead {
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
                agents(stub, home.path(), dir.path(), spill),
                gh_stub(NO_GITHUB),
                cluster.clone(),
                nudges.clone(),
                probing(dir.path()),
            ),
        };

        let listening = tokio::spawn(listener.serving(peer::router(
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
            listening,
            _dir: dir,
            _home: home,
            elsewhere,
        }
    }

    /// Hold a Nudge stream to every member, which is what a running server
    /// spawns at the start: it is what refreshes this device's mirrors of a
    /// member's Profiles and what keeps its merged sidebar fresh.
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

    /// The machine switched off: the listener stops answering, and every dial at
    /// it from here on is a device that is not there.
    fn stops_answering(&self) {
        self.listening.abort();
    }

    /// Write `other` down as one of this device's members.
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

    /// A repository of this device's own, really made and really registered.
    ///
    /// Two of these, one per device, are what the match settles between: neither
    /// has an origin, so what says they are one repository is the directory's own
    /// name.
    async fn repo(&self) -> PathBuf {
        self.repo_called(REPOSITORY).await
    }

    /// The same again under a name of its own, which is what a **Companion Repo**
    /// is: a second registered repository beside the Conversation's own.
    async fn repo_called(&self, name: &str) -> PathBuf {
        let path = repository(self.elsewhere.path().join(name));

        let registered = press(
            &self.workbench,
            "/api/ui/repos",
            Some(&serde_json::json!({ "path": path }).to_string()),
        )
        .await;

        assert!(registered.contains("Added"), "registering {name}");

        path
    }

    /// The same repository again as a **clone** of `from`, registered here.
    ///
    /// Which is the ordinary shape of a cluster: two machines holding the same
    /// repository, with history in common. What that history is for is the
    /// bundle — a branch packed against the tips the far end already holds is
    /// the branch and not the history under it.
    ///
    /// **And the clone's `origin` is taken off it**, because a clone has one and
    /// the machine it was cloned from has none: a Repo with an origin never
    /// matches one without, so the two would be two repositories. What is left
    /// is two clones nobody has pushed anywhere, matched by name — which is
    /// what `Self::repo` sets up on both sides, with the history added.
    async fn cloned_from(&self, from: &Path) -> PathBuf {
        self.cloned_from_called(from, REPOSITORY).await
    }

    /// And a clone under a name of its own, for a **Companion Repo**: a
    /// Companion's branch is bundled against what the far end holds of *that*
    /// repository, so the two ends need a history in common there too.
    async fn cloned_from_called(&self, from: &Path, name: &str) -> PathBuf {
        let path = self.elsewhere.path().join(name);

        git(
            self.elsewhere.path(),
            &["clone", &from.display().to_string(), name],
        );
        git(&path, &["remote", "remove", "origin"]);
        git(&path, &["config", "user.email", "test@verkstead.invalid"]);
        git(&path, &["config", "user.name", "Verkstead Test"]);

        let registered = press(
            &self.workbench,
            "/api/ui/repos",
            Some(&serde_json::json!({ "path": path }).to_string()),
        )
        .await;

        assert!(registered.contains("Added"), "registering the clone");

        path
    }

    /// Where that Conversation's work is checked out, as this device's own
    /// record has it.
    async fn worktree(&self, conversation: i64) -> PathBuf {
        store::load_conversation(&self.pool, conversation)
            .await
            .unwrap()
            .expect("the Conversation is on this device")
            .worktree
            .expect("work past drafting has a Worktree")
    }

    /// And where its **Companion** in the Repo called `name` is checked out, as
    /// this device's own record has it.
    async fn companion_worktree(&self, conversation: i64, name: &str) -> PathBuf {
        self.companion_row(conversation, name)
            .await
            .worktree
            .unwrap_or_else(|| panic!("the companion {name} has a checkout"))
    }

    /// The Companion row itself, which is what says the mode a sandbox would bind
    /// its directory under.
    async fn companion_row(&self, conversation: i64, name: &str) -> store::Companion {
        store::companions(&self.pool, conversation)
            .await
            .unwrap()
            .into_iter()
            .find(|companion| companion.repo.name == name)
            .unwrap_or_else(|| panic!("{name} is a companion of this Conversation"))
    }

    /// And where it is on this machine, which is where its own `git config` is.
    fn repo_path(&self, name: &str) -> PathBuf {
        self.elsewhere.path().join(name)
    }

    /// The id this device's registry gave the Repo called `name`.
    async fn repo_row(&self, name: &str) -> i64 {
        let repos: Vec<verkstead_render::RepoEntry> =
            reading(&self.workbench, "/api/ui/repos").await;

        repos
            .iter()
            .find(|repo| repo.name == name)
            .unwrap_or_else(|| panic!("{name} is registered here: {repos:#?}"))
            .id
    }

    /// An account of this device's own, really on disk, with a Profile saved over
    /// it the way the form saves one.
    async fn account(&self) -> i64 {
        let under = self.elsewhere.path().join(ACCOUNT);
        let claude_dir = under.join(".claude");
        let config_file = under.join(".claude.json");

        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(&config_file, "{}\n").unwrap();
        std::fs::write(
            claude_dir.join(".credentials.json"),
            "{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-the-desk\"}}",
        )
        .unwrap();

        let said = press(
            &self.workbench,
            PROFILES,
            Some(
                &serde_json::json!({
                    "name": ACCOUNT,
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

        assert_eq!(said, "\"Saved\"", "saving the account");

        self.profile_called(ACCOUNT).await.id
    }

    /// The row of this device's Profiles with that name, once there is one —
    /// which on B is the **mirror** its refresher writes off A's list.
    async fn profile_called(&self, name: &str) -> ProfileEntry {
        let mut last = Vec::new();

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                last = reading(&self.workbench, PROFILES).await;

                if let Some(row) = last
                    .iter()
                    .find(|row: &&ProfileEntry| row.name.as_deref() == Some(name))
                {
                    return row.clone();
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("no Profile called {name} was ever listed: {last:#?}"))
    }

    /// A Conversation of this device's, written the way the composer writes one
    /// and left a draft — which is where a file is put on one: a Conversation
    /// takes attachments while it is drafting and no longer.
    ///
    /// What starts it is [`Self::grills`], one press along.
    async fn drafting_under(&self, profile: i64) -> i64 {
        let repos: Vec<verkstead_render::RepoEntry> =
            reading(&self.workbench, "/api/ui/repos").await;

        let started = press(
            &self.workbench,
            SIDEBAR,
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

        conversation
    }

    /// And the press that starts it, which is where the draft stops being one.
    async fn grills(&self, conversation: i64) {
        let grilling = press(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/grill"),
            None,
        )
        .await;

        assert_eq!(grilling, "\"Started\"", "starting the grilling");
    }

    /// Put a file on a drafting Conversation, the way the paperclip does, and
    /// answer with the id the record gave it.
    async fn attaches(&self, conversation: i64, name: &str, bytes: &[u8]) -> i64 {
        let attached = self
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/ui/conversations/{conversation}/attachments/{name}"
                    ))
                    .body(Body::from(bytes.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = attached.status();
        let body = attached.into_body().collect().await.unwrap().to_bytes();
        let said = String::from_utf8_lossy(&body).into_owned();

        assert_eq!(status, StatusCode::OK, "attaching {name}: {said}");
        assert!(said.contains("Attached"), "attaching {name}: {said}");

        self.view(conversation)
            .await
            .attachments
            .iter()
            .find(|attachment| attachment.name == name)
            .expect("the file is on the record")
            .id
    }

    /// The bytes of one of its files, read the way the browser downloads one.
    async fn attached_bytes(&self, conversation: i64, attachment: i64) -> Vec<u8> {
        let read = self
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/ui/conversations/{conversation}/attachments/{attachment}/bytes"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            read.status(),
            StatusCode::OK,
            "reading attachment {attachment}"
        );

        read.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec()
    }

    /// Ask a Question Set the way a session's CLI does: to the base URL its
    /// sandbox was given, which is this Conversation's own.
    async fn asks(&self, conversation: i64, yaml: &str) -> i64 {
        self.asking(conversation, yaml, false).await
    }

    /// And a **Deferred Ask**: the same Set, with nothing waiting on the Answer.
    ///
    /// Which is the one kind that survives its session. A relaunch locks every
    /// Set the gone session was idling on — the human would be answering into
    /// nothing — and leaves a Deferred Ask exactly where it stands, nobody having
    /// been behind it to begin with. See `server::sets::Open`.
    async fn defers(&self, conversation: i64, yaml: &str) -> i64 {
        self.asking(conversation, yaml, true).await
    }

    /// Both, and the query the CLI spells the difference with.
    async fn asking(&self, conversation: i64, yaml: &str, deferred: bool) -> i64 {
        let query = if deferred { "?deferred=true" } else { "" };

        let asked = self
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{conversation}/api/v1/sets{query}"))
                    .header(CONTENT_TYPE, "application/yaml")
                    .body(Body::from(yaml.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = asked.status();
        let body = asked.into_body().collect().await.unwrap().to_bytes();
        let said = String::from_utf8_lossy(&body).into_owned();

        assert_eq!(status, StatusCode::CREATED, "asking a Set: {said}");

        let created: verkstead_schema::SetCreated = serde_saphyr::from_str(&said).unwrap();

        created.id
    }

    /// Answer one of its Question Sets the way the human's own device does, over
    /// the endpoint a session's CLI is waiting on.
    ///
    /// What it is for is the **digest**: a grilling started again is primed with
    /// what has already been settled, and nothing is settled until a Set has an
    /// Answer.
    async fn answers(&self, conversation: i64, set: i64, yaml: &str) {
        let answered = self
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/conversations/{conversation}/api/v1/sets/{set}/response"
                    ))
                    .header(CONTENT_TYPE, "application/yaml")
                    .body(Body::from(yaml.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = answered.status();
        let bytes = answered.into_body().collect().await.unwrap().to_bytes();
        let said = String::from_utf8_lossy(&bytes).into_owned();

        assert!(status.is_success(), "answering Set {set}: {status} {said}");
    }

    /// The same over the raw status, for the times a refusal is the answer being
    /// asked after.
    async fn answering(&self, conversation: i64, set: i64, yaml: &str) -> StatusCode {
        self.workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/conversations/{conversation}/api/v1/sets/{set}/response"
                    ))
                    .header(CONTENT_TYPE, "application/yaml")
                    .body(Body::from(yaml.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }

    /// running.
    ///
    /// The press that writes the stop rather than asking for one — an ordinary
    /// Stop is a request the next launch answers, and a grilling seen out has no
    /// next launch to answer it. What it is here for is the arrival: a stop
    /// somebody *decided on* crosses with the record and is left exactly where it
    /// stands over there, so nothing is started on the far end and nothing locks
    /// what the human has still to answer.
    async fn stops_at_once(&self, conversation: i64) {
        let said = press(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/force-stop"),
            None,
        )
        .await;

        assert_eq!(said, "\"Stopped\"", "pressing Force stop");
    }

    /// Press *Transfer to…* for `device`, and answer with what it said.
    async fn transfers(&self, conversation: i64, device: &str) -> String {
        press(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/transfer/{device}"),
            None,
        )
        .await
    }

    /// One of its Conversations as its own browser reads it.
    async fn view(&self, conversation: i64) -> ConversationView {
        reading(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}"),
        )
        .await
    }

    /// The same, polled until it says what the test is waiting for — or a panic
    /// saying what it says instead.
    async fn view_saying(
        &self,
        conversation: i64,
        what: impl Fn(&ConversationView) -> bool,
    ) -> ConversationView {
        let deadline = Instant::now() + WAITING;

        loop {
            let drawn = self.view(conversation).await;

            if what(&drawn) {
                return drawn;
            }

            assert!(
                Instant::now() < deadline,
                "Conversation {conversation} never said it. It says: \
                 state {:?}, working {}, transferring {:?}, transferred {:?}, stopped {:?}",
                drawn.state,
                drawn.working,
                drawn.transferring,
                drawn.transferred,
                drawn.blocked_on,
            );

            tokio::time::sleep(LOOKING).await;
        }
    }

    /// The Conversations this device holds rows for of its own, which is a
    /// different question from its sidebar: that one is the cluster's merged
    /// list, and what is asked here is whether anything was written *here*.
    async fn own_rows(&self) -> Vec<verkstead_store::ConversationRow> {
        store::conversations(&self.pool, true).await.unwrap()
    }

    /// The sidebar as this device's own browser reads it, which is where a Rank
    /// and a birth key ride out.
    async fn sidebar(&self) -> Vec<ConversationEntry> {
        reading(&self.workbench, SIDEBAR).await
    }

    /// The same, polled until it says it — a merged list being two hops from
    /// whatever changed on the far end.
    async fn sidebar_saying(
        &self,
        what: impl Fn(&[ConversationEntry]) -> bool,
    ) -> Vec<ConversationEntry> {
        let mut last = Vec::new();

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                last = self.sidebar().await;

                if what(&last) {
                    return last.clone();
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("the sidebar never said it: {last:#?}"))
    }

    /// Wait until a session is running on that Conversation and has printed
    /// `saying`, which is what says a turn is genuinely in flight.
    async fn printed(&self, conversation: i64, saying: &str) {
        let deadline = Instant::now() + WAITING;
        let mut said = String::new();

        loop {
            if let Some(text) = self.capture(conversation).await {
                said = text;
            }

            if said.contains(saying) {
                return;
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
        let drawn = self.view(conversation).await;

        let event = drawn.timeline.iter().find_map(|event| match event {
            TimelineEvent::AgentOutput(output) => Some(output.id),
            _ => None,
        })?;

        let capture: verkstead_render::Capture = reading(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/capture/{event}"),
        )
        .await;

        Some(capture.text)
    }

    /// Every Question Set on its Timeline, by the id **this** device numbered
    /// each, in the order they were asked.
    ///
    /// Which is the whole point of reading them off the Timeline rather than
    /// remembering what they were called on the machine they came from: every
    /// Verkstead issues its own, and a wait is opened on the one it has here.
    async fn sets_on(&self, conversation: i64) -> Vec<i64> {
        self.view(conversation)
            .await
            .timeline
            .iter()
            .filter_map(|event| match event {
                TimelineEvent::QuestionSet(asked) => Some(asked.set_id),
                _ => None,
            })
            .collect()
    }

    /// Where this device keeps that Conversation's attached files, which is the
    /// directory its own sessions are given — see the server's `attachments`.
    /// Where this device keeps that Conversation's attached files, which is the
    /// directory its own sessions are given — see the server's `attachments`.
    fn attachments_directory(&self, conversation: i64) -> PathBuf {
        self._dir
            .path()
            .join("attachments")
            .join(conversation.to_string())
    }

    /// Every session on that Conversation's Timeline, by the Event each printed
    /// into.
    ///
    /// In Timeline order, so the last of them is the newest: what a Conversation
    /// that has arrived here grows is a second one, started by this device's own
    /// Resume beside the one that crossed with the record.
    async fn sessions_on(&self, conversation: i64) -> Vec<i64> {
        self.view(conversation)
            .await
            .timeline
            .iter()
            .filter_map(|event| match event {
                TimelineEvent::AgentOutput(output) => Some(output.id),
                _ => None,
            })
            .collect()
    }

    /// What one of them printed, by the Event it printed into.
    async fn capture_of(&self, conversation: i64, event: i64) -> String {
        let capture: verkstead_render::Capture = reading(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/capture/{event}"),
        )
        .await;

        capture.text
    }

    /// The newest session's Capture, once it says `saying` — or a panic saying
    /// what it says instead.
    ///
    /// The newest rather than the first, which is what [`Self::capture`] reads:
    /// on the far end of a move the first is the session that crossed with the
    /// record, and what this device started for itself is the one after it.
    async fn latest_capture_saying(&self, conversation: i64, saying: &str) -> String {
        let deadline = Instant::now() + WAITING;
        let mut said = String::new();

        loop {
            if let Some(event) = self.sessions_on(conversation).await.last().copied() {
                said = self.capture_of(conversation, event).await;
            }

            if said.contains(saying) {
                return said;
            }

            assert!(
                Instant::now() < deadline,
                "no session on Conversation {conversation} ever printed {saying:?}. \
                 The newest printed: {said:?}",
            );

            tokio::time::sleep(LOOKING).await;
        }
    }

    /// Where the account called `name` keeps its login on this device, which is a
    /// real file under a real account directory.
    fn login_of(&self, name: &str) -> PathBuf {
        self.elsewhere
            .path()
            .join(name)
            .join(".claude/.credentials.json")
    }

    /// And what it holds, where the account is at home here.
    fn login_at_home(&self, name: &str) -> String {
        std::fs::read_to_string(self.login_of(name)).unwrap_or_default()
    }

    /// And every Notice on its Timeline, which is where Verkstead says what it
    /// did on its own account — a move that failed among them.
    /// And every Notice on its Timeline, which is where Verkstead says what it
    /// did on its own account — a move that failed among them.
    async fn notices(&self, conversation: i64) -> Vec<String> {
        self.view(conversation)
            .await
            .timeline
            .iter()
            .filter_map(|event| match event {
                TimelineEvent::Notice(notice) => Some(notice.html.clone()),
                _ => None,
            })
            .collect()
    }
}

/// The streams one device is holding, given back when the test drops it.
struct Holding(JoinHandle<()>);

impl Drop for Holding {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// What stands where claude goes: a session that says it is grilling, waits at
/// the gate the test opens, and stops.
///
/// The wait is the whole of what this suite needs of an agent. A press that
/// arrived after the session had gone would prove nothing about the promise the
/// press makes, which is that a turn part way through runs to its own end.
fn waits_at(gate: &Path) -> String {
    format!(
        r#"
printf 'grilling\r\n'
while [ ! -f {gate} ]; do sleep 0.05; done
printf 'the turn is over\r\n'
"#,
        gate = quoted(gate),
    )
}

/// The same again with a login refreshed before the wait, which is the one thing
/// a harness does to the account it is running as.
///
/// What it is for is the **ordering**: a session away from home writes the login
/// it now holds back to the device that account is at home on as it ends, and a
/// move that overtook that write-back would leave the far end launching under a
/// login this machine had not finished returning. So the session changes one, and
/// the far end is asked what it is running under.
fn refreshes_its_login_at(gate: &Path, token: &str) -> String {
    format!(
        "printf '{{\"claudeAiOauth\":{{\"accessToken\":\"%s\"}}}}' '{token}' \
> \"$HOME/.claude/.credentials.json\"\nprintf 'refreshed the login\\r\\n'\n{}",
        waits_at(gate),
    )
}

/// And what stands where claude goes **on the far end**: the session that device's
/// own Resume starts once the work has arrived.
///
/// It says three things, and each is something only the arrival could have given
/// it. That it is running at all, which is the Resume; the **prompt** it was
/// launched on, which is the record re-read on this machine — a grilling again
/// carries the Brief and what has already been settled; and the **login** it is
/// running under, which is the account the source was using, written home as that
/// session ended.
fn re_primed() -> String {
    r#"
printf 're-primed\r\n'
printf 'prompt=%s\r\n' "$2"
printf 'login=%s\r\n' "$(cat "$HOME/.claude/.credentials.json" 2>/dev/null)"
"#
    .to_owned()
}

/// Set `core.autocrlf` on a repository, the way the machine it is on would have
/// it — and respell whatever is checked out of it so the trees agree with the
/// switch.
///
/// **Which is what an operating system amounts to here.** A Windows checkout
/// keeps CRLF in the working tree and LF in the index; a Unix one keeps LF in
/// both. So two repositories set opposite ways are the two machines as far as
/// anything about a patch can tell, and the crossing can be made on one of them.
fn keeping_endings(repo: &Path, autocrlf: &str, worktrees: &[&Path]) {
    git(repo, &["config", "core.autocrlf", autocrlf]);

    for worktree in worktrees {
        git(worktree, &["checkout", "HEAD", "--", "."]);
    }
}

/// A path as a shell script can carry it.
fn quoted(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

/// The agent a session on this device runs, and everything else a launch is
/// equipped out of.
fn agents(stub: &str, home: &Path, data_dir: &Path, spill: &Path) -> Agents {
    Agents::running(
        vec!["/bin/sh".to_owned(), "-c".to_owned(), stub.to_owned()],
        Homes::on(Platform::HERE, home.to_owned(), data_dir),
        Reachable::at(LISTENING),
        // The gate the stub waits at is outside the Worktree, so the sandbox has
        // to be able to see it.
        SandboxConfig::resolve(&[spill.display().to_string()]).unwrap(),
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

/// A `gh` that is a shell script, so nothing here reaches GitHub.
fn gh_stub(script: &str) -> Gh {
    Gh::running(vec![
        "/bin/sh".to_owned(),
        "-c".to_owned(),
        script.to_owned(),
        "gh".to_owned(),
    ])
}

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422)
}

/// The machine a device here is judged on: one directory on the `PATH` a session
/// would search, holding a program for every harness.
///
/// **Stated rather than read off the runner**, as the suites beside this one
/// state theirs: whether a harness is on the far end is what `tests/preflight.rs`
/// is about, and here it must not be what decides whether a press can be made.
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

/// A and B, linked both ways, each with the repository registered — and B
/// holding the streams that write its mirror of A's account, which is what a
/// Pairing arriving there resolves against.
///
/// The Conversation is grilling on A under that account, with its session
/// waiting at `gate`.
async fn ready_to_move(gate: &Path, spill: &Path) -> (Verkstead, Verkstead, Holding, i64) {
    let (a, b, holding, conversation) = ready_to_draft(gate, spill).await;

    a.grills(conversation).await;

    (a, b, holding, conversation)
}

/// The same again with B's repository a **clone** of A's, which is what the git
/// leg is about: two machines with history in common, so that what crosses is
/// the branch rather than everything under it.
///
/// Every test about the branch and the working changes starts here.
async fn ready_to_move_from_a_clone(
    gate: &Path,
    spill: &Path,
) -> (Verkstead, Verkstead, Holding, i64) {
    let (a, b, holding, conversation) = drafted(gate, spill, Repositories::Cloned).await;

    a.grills(conversation).await;

    (a, b, holding, conversation)
}

/// The same again with a **Companion Repo** beside the Conversation's own, in
/// `mode`, and a clone of it on B.
///
/// Every test about a Companion over the link starts here: the second repository
/// is registered on both machines with a history in common, the Companion is put
/// on the Conversation while it is still drafting — which is the only time a
/// Companion may be added — and the grill start is what cuts its checkout beside
/// the Conversation's own.
async fn ready_to_move_alongside(
    gate: &Path,
    spill: &Path,
    mode: store::CompanionMode,
) -> (Verkstead, Verkstead, Holding, i64) {
    let (a, b, holding, conversation) = drafted(gate, spill, Repositories::Cloned).await;

    let theirs = a.repo_called(COMPANION).await;
    b.cloned_from_called(&theirs, COMPANION).await;

    let beside = a.repo_row(COMPANION).await;

    assert_eq!(
        store::add_companion(&a.pool, conversation, beside)
            .await
            .unwrap(),
        store::Adding::Added,
        "putting the companion on the draft",
    );

    // Read-only is what a companion starts as, so only the other way needs
    // saying — and saying it is what makes a branch be cut for it at all.
    if mode == store::CompanionMode::ReadWrite {
        assert_eq!(
            store::configure_companion(&a.pool, conversation, beside, store::Change::Mode(mode))
                .await
                .unwrap(),
            store::Configured::Saved,
            "setting the companion read-write",
        );
    }

    a.grills(conversation).await;

    (a, b, holding, conversation)
}

/// Whether the two machines' repositories share a history.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Repositories {
    /// Two repositories made separately, sharing a name and nothing else —
    /// which is every test about the record, where the history is beside the
    /// point.
    Apart,

    /// B's is a clone of A's, which is what a cluster holding one repository
    /// looks like.
    Cloned,
}

/// The same, stopped at the draft: the two machines linked and the Conversation
/// written, with the press that starts it still to come.
///
/// Which is where a file is put on one — a Conversation takes attachments while
/// it is drafting and no longer — and so is where the test about an Attachment
/// crossing begins.
async fn ready_to_draft(gate: &Path, spill: &Path) -> (Verkstead, Verkstead, Holding, i64) {
    drafted(gate, spill, Repositories::Apart).await
}

/// Both of the above, and the one thing that differs between them.
async fn drafted(
    gate: &Path,
    spill: &Path,
    repositories: Repositories,
) -> (Verkstead, Verkstead, Holding, i64) {
    drafted_running(&waits_at(gate), spill, repositories).await
}

/// The same with the session on A saying what this test needs of it, which is the
/// one thing that differs between the setups.
///
/// **B's is never the same script.** Nothing was ever launched on the far end
/// before this stage; now its own Resume starts a session the moment the work
/// lands, and a suite whose two machines printed the same words could not say
/// which of them had run. See [`re_primed`].
async fn drafted_running(
    stub: &str,
    spill: &Path,
    repositories: Repositories,
) -> (Verkstead, Verkstead, Holding, i64) {
    let a = Verkstead::running(A, stub, spill).await;
    let b = Verkstead::running(B, &re_primed(), spill).await;

    a.linked_to(&b.device, B_MACHINE, B_OS, vec![b.at()]).await;

    let (machine, os) = this_machine();
    b.linked_to(&a.device, &machine, &os, vec![a.at()]).await;

    let theirs = a.repo().await;

    match repositories {
        Repositories::Apart => b.repo().await,
        Repositories::Cloned => b.cloned_from(&theirs).await,
    };

    let account = a.account().await;

    // B's mirror of that account, which is stage 08's machinery and what the
    // Pairings arriving there are resolved against: a Profile is named across a
    // cluster by the device it is at home on and its id there.
    let holding = b.holding();
    b.profile_called(ACCOUNT).await;

    let conversation = a.drafting_under(account).await;

    (a, b, holding, conversation)
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

/// Whether the checkout at `dir` holds no branch at all, which is the shape a
/// read-only Companion is given: `git symbolic-ref HEAD` answers with nothing and
/// fails, there being no name for it to answer with.
fn detached(dir: &Path) -> bool {
    !Command::new("git")
        .args(["symbolic-ref", "--quiet", "HEAD"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("git should be on the PATH for these tests")
        .success()
}

/// And run git in `dir` for what it says, which is how this suite reads a
/// repository back: where a branch stands, and what a checkout is on.
fn git_says(dir: &Path, args: &[&str]) -> String {
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
        dir.display(),
    );

    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// Whether a row of a merged list is this device's own copy of Conversation
/// `id`.
///
/// Every row of a merged list says whose it is, and this device's own say so by
/// naming no device inside — see `RowDevice`, whose `null` id is the reading
/// device itself.
fn ours(row: &ConversationEntry, id: i64) -> bool {
    row.id == id && row.device.as_ref().is_none_or(|whose| whose.id.is_none())
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

/// **The press waits for the turn to end**, which is the whole of what it
/// promises: the session running when it is made runs to its own end, nothing is
/// started after it, and the work goes once there is nothing left running.
///
/// And from the press until it lands the Conversation says where it is going,
/// which is what the head of its Timeline reads.
#[tokio::test]
async fn a_press_over_a_running_session_waits_for_the_turn_to_end() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    // A turn genuinely in flight: the session is up, in a real sandbox, part way
    // through what it was launched for.
    a.printed(conversation, "grilling").await;

    assert_eq!(
        a.transfers(conversation, B).await,
        "\"Transferring\"",
        "the press is taken",
    );

    // What the head of the Timeline reads from here: the machine the work is
    // going to, by the name the human gave it.
    let going = a.view(conversation).await;
    assert_eq!(going.transferring.as_deref(), Some(B_MACHINE));
    assert!(
        going.transferred.is_none(),
        "and nothing has moved: this copy is still the record",
    );

    // Nothing is cut short. The session is still running and still printing,
    // because what the press asked for is the turn's end rather than the
    // session's.
    assert!(
        going.working,
        "the session the press arrived over is running"
    );
    assert!(
        b.own_rows().await.is_empty(),
        "and nothing has reached the far end while it is",
    );

    // The turn ends.
    std::fs::write(&gate, "go").unwrap();

    // And the move runs: B holds the work, and A's copy says where it went.
    let handed = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await;

    let there = handed.transferred.expect("the mark the move wrote");
    assert_eq!(there.device, B, "the work is on B");

    assert_eq!(
        handed.transferring, None,
        "and the Conversation is no longer going anywhere: it has gone",
    );

    let arrived = b.view(there.id).await;
    assert_eq!(
        arrived.branch, handed.branch,
        "the branch the work is on crossed with it",
    );
}

/// **What arrives is the Conversation**: the state it was in, the Repo the
/// matching settled, its Pairings as B's own ids, and the two things that are
/// the cluster's rather than either machine's — the Rank and the birth key.
#[tokio::test]
async fn the_conversation_arrives_in_the_state_it_was_in() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    // The turn is over before the press is made, which is the other way one
    // arrives: there is nothing to see out, and the move runs where it stands.
    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    // What the work was, before it went: the row's Rank and the key it was born
    // under, read off this device's own store.
    let rank = store::conversation_rank(&a.pool, conversation)
        .await
        .unwrap()
        .expect("every Conversation is ranked as it is started");
    let born = store::birth(&a.pool, conversation)
        .await
        .unwrap()
        .expect("every Conversation is stamped as it is started");

    let branch = a.view(conversation).await.branch;

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    let arrived = b.view(there).await;

    assert_eq!(
        arrived.state,
        Lifecycle::Grilling,
        "it arrives in the state it was in",
    );
    assert_eq!(arrived.branch, branch, "on the branch the work is on");
    assert_eq!(
        arrived.repo.path,
        b.elsewhere.path().join(REPOSITORY).display().to_string(),
        "under B's own Repo, which is the one the matching settled — and not a \
         path of A's",
    );

    // The Pairings are ids of B's own: the account is at home on A, so what B
    // holds is a mirror of it, and that is what the row names.
    let mirror = b.profile_called(ACCOUNT).await;
    let pairing = arrived
        .grilling_pairing
        .as_ref()
        .expect("the grilling Pairing crossed with it");

    assert_eq!(pairing.profile.id, mirror.id, "B's own id for that account");
    assert_eq!(
        pairing.model.as_deref(),
        Some(MODEL),
        "and the model with it"
    );
    assert_eq!(
        arrived
            .implementation_pairing
            .as_ref()
            .map(|pairing| pairing.profile.id),
        Some(mirror.id),
        "and the implementation's, which is the same account here",
    );

    // And the two the cluster names it by, verbatim: the Rank carries the device
    // that issued it and the birth key says where the work was drafted, so
    // neither is B's to invent.
    let row = b
        .sidebar_saying(|rows| rows.iter().any(|row| ours(row, there)))
        .await
        .into_iter()
        .find(|row| ours(row, there))
        .expect("B's own row for the work that arrived");

    assert_eq!(row.rank, rank, "it keeps its place in the merged order");
    assert_eq!(
        row.born,
        born.key(),
        "and the key it was born under, which is what says the two copies are \
         one piece of work",
    );
}

/// **The source is marked only once the far end has confirmed**, and what it
/// keeps is a tombstone: the merged list draws the work once, and the copy's own
/// URL leads to wherever the record is now.
#[tokio::test]
async fn the_source_keeps_a_copy_that_leads_to_the_live_record() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding_there, conversation) = ready_to_move(&gate, spill.path()).await;

    // A holds its own streams too, so that its sidebar is the cluster's rather
    // than its own rows alone — which is what a merged list is.
    let _holding_here = a.holding();

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    assert_eq!(
        a.sidebar().await.len(),
        1,
        "one row before the move, which is A's own",
    );

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let handed = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await;

    let there = handed.transferred.expect("the mark the move wrote");

    assert_eq!(there.device, B);
    assert_eq!(
        b.view(there.id).await.transferred,
        None,
        "and B's copy is the record, wearing no mark of its own",
    );

    // One row, and it is B's: the tombstone comes off the merged list and the
    // live copy stands in its place, at the rank the work has always sat at.
    let drawn = a
        .sidebar_saying(|rows| {
            rows.iter().any(|row| {
                row.id == there.id
                    && row.device.as_ref().and_then(|whose| whose.id.as_deref()) == Some(B)
            })
        })
        .await;

    assert_eq!(drawn.len(), 1, "the work is drawn once: {drawn:#?}");
    assert_eq!(drawn[0].id, there.id, "and the row drawn is B's copy");
}

/// **A move that fails leaves the source live and stopped**: nothing is marked,
/// the Worktree is where it was, and the Notice names the machine and what went
/// wrong.
///
/// The machine goes off between the press and the crossing, which is the
/// ordinary way for this to happen: the preflight is a reading of a moment, and
/// a laptop's lid shuts.
#[tokio::test]
async fn a_move_that_fails_leaves_the_source_live_and_stopped() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    // The lid shuts while the turn is still running, so the press was made
    // against a machine that answered and the move meets one that does not.
    b.stops_answering();

    let worktree = a
        .view(conversation)
        .await
        .worktree
        .expect("the grilling cut a Worktree")
        .path;

    std::fs::write(&gate, "go").unwrap();

    let stopped = a
        .view_saying(conversation, |drawn| drawn.blocked_on.is_some())
        .await;

    assert_eq!(
        stopped.transferred, None,
        "nothing was marked: this copy is still the record",
    );
    assert_eq!(
        stopped.transferring, None,
        "and it is not on its way anywhere any more",
    );
    assert!(
        stopped.ready_to_resume,
        "it is stopped, which is a press away from being driven again",
    );

    let notices = a.notices(conversation).await;
    let said = notices
        .last()
        .expect("a stop writes a Notice saying what happened");

    assert!(
        said.contains(B_MACHINE),
        "the Notice names the machine: {said}",
    );

    assert!(
        std::path::Path::new(&worktree).is_dir(),
        "and the Worktree is where it was: {worktree}",
    );

    assert!(
        b.own_rows().await.is_empty(),
        "and nothing was left on the far end",
    );
}

/// **The Timeline on the far end reads as it read here**: the same Events in the
/// same order, each drawing the card it drew, with the Captures and the session
/// names behind them.
///
/// Read as the browser reads it, with the ids taken out — because the ids are the
/// whole of what may differ. Every Verkstead numbers its own rows, so an Event on
/// B is a different number from the Event on A it is a copy of; everything else
/// on the card is the record, and any of it arriving changed would be the record
/// changing as it moved.
///
/// And the rows keyed by an Event rather than by the Conversation are the ones
/// worth naming: a Capture, a Transcript and a session's name each hang off one,
/// so any of them left pointing at the number it had on A would be a card with
/// nothing behind it.
#[tokio::test]
async fn the_timeline_on_the_far_end_reads_as_it_read_here() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;

    // A Set on the way, so the Timeline has a card of every kind the grilling
    // writes rather than only the ones a start does.
    a.asks(conversation, ASKED).await;

    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let here = a.view(conversation).await;
    let said = a.capture(conversation).await;
    let names = session_names(&a.pool).await;

    assert!(
        here.timeline.len() >= 4,
        "the Timeline this is read against has the Brief, the move, the session \
         and the Set on it: {:#?}",
        here.timeline,
    );

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    // What crossed is the front of B's Timeline, and what B adds to it is its
    // own: the Notice saying where the work came from, and the session its Resume
    // started. So the wait is for that Notice — the first thing written here that
    // was not carried — and what is read against A's is everything in front of
    // it.
    let arrived = b
        .view_saying(there, |drawn| drawn.timeline.len() > here.timeline.len())
        .await;

    assert_eq!(
        cards(&arrived.timeline[..here.timeline.len()]),
        cards(&here.timeline),
        "the Timeline reads as it did, card for card",
    );

    assert_eq!(
        b.capture(there).await,
        said,
        "and what the session printed is behind the Event it printed into",
    );

    let named = session_names(&b.pool).await;

    assert_eq!(
        &named[..names.len()],
        &names[..],
        "as is the name Verkstead ran that session under: {named:#?}",
    );

    let here_lines = transcripts(&a.pool).await;
    let there_lines = transcripts(&b.pool).await;

    assert_eq!(
        &there_lines[..here_lines.len()],
        &here_lines[..],
        "and the log the session kept of itself, line for line",
    );
}

/// **A Question Set left open is answerable where the work now is**, and its
/// Answers reach whatever is waiting there.
///
/// The wait is opened on B through the very endpoint a session's CLI opens one
/// on, over B's own Conversation id and B's own Set id — which is what the whole
/// renumbering is for. A Set whose id had not moved with it would be a wait
/// nothing could open, and one answered into A would be an Answer reaching a
/// machine the work has left.
///
/// **Stopped before it is moved**, which is the shape in which a Set really
/// outlives a move. The run is stopped by a press, that stop crosses with the
/// record, and the far end leaves a stop somebody decided on exactly where it
/// stands — so nothing is started over there and nothing locks what the human
/// has still to answer. A grilling the far end *does* start again is the other
/// half of the same rule, and it is
/// `the_set_a_gone_session_was_waiting_on_is_locked_where_the_work_lands`'s.
#[tokio::test]
async fn a_set_left_open_is_answerable_where_the_work_now_is() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;
    a.asks(conversation, ASKED).await;

    // The human pulling the brake, which is what makes the question theirs to
    // answer wherever the work goes: a stop somebody decided on waits for a
    // press, and an arrival is not somebody deciding differently about it.
    a.stops_at_once(conversation).await;

    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    let set = *b
        .sets_on(there)
        .await
        .first()
        .expect("the Set crossed with the work, under an id of B's own");

    // A session on B waiting for its Answers, which is what the endpoint is:
    // held open until the Set is settled. Opened before anything answers, so
    // what ends it is the answering rather than a read that was already true.
    let waiting = tokio::spawn({
        let workbench = b.workbench.clone();

        async move {
            let answered = workbench
                .oneshot(
                    Request::builder()
                        .uri(format!(
                            "/conversations/{there}/api/v1/sets/{set}/response?hold=20"
                        ))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            let status = answered.status();
            let bytes = answered.into_body().collect().await.unwrap().to_bytes();

            (status, String::from_utf8_lossy(&bytes).into_owned())
        }
    });

    // And the human answering it on the machine the work is on now.
    let taken = b
        .workbench
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/conversations/{there}/api/v1/sets/{set}/response"))
                .header(CONTENT_TYPE, "application/yaml")
                .body(Body::from(ANSWERED))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = taken.status();
    let bytes = taken.into_body().collect().await.unwrap().to_bytes();
    let said = String::from_utf8_lossy(&bytes).into_owned();

    assert!(status.is_success(), "answering it on B: {status} {said}");

    let (status, handed) = tokio::time::timeout(WAITING, waiting)
        .await
        .expect("the wait on B ended once the Set was answered")
        .unwrap();

    assert_eq!(
        status,
        StatusCode::OK,
        "the wait was handed a Response: {handed}"
    );
    assert!(
        handed.contains("shared between instances"),
        "and it is the Answer that was given: {handed}",
    );
}

/// **And the Set a session that has gone was waiting on is locked where the work
/// lands**, by the very Resume that starts the work again there.
///
/// Which is the other side of the rule above, and it is not about the move at
/// all: a grilling started again locks every Set the dead session was *idling
/// on*, because the reader has gone and the human would be answering into
/// nothing — and after a move the reader has gone in the most literal way there
/// is, the machine it was on having handed the work over. The session the far end
/// starts is primed with what was answered and asks again what it still needs.
///
/// **And the Deferred Ask beside it is left where it stands**, nobody having been
/// behind it to begin with: it crosses, it lands open, and its Answers go into the
/// prompt of a later session of this Conversation — which is now a session on
/// another machine.
#[tokio::test]
async fn the_set_a_gone_session_was_waiting_on_is_locked_where_the_work_lands() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;

    let idled = a.asks(conversation, ASKED).await;
    a.defers(conversation, ASKED).await;

    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    let crossed = b.sets_on(there).await;

    assert_eq!(
        crossed.len(),
        2,
        "both Sets crossed with the work, under ids of B's own",
    );

    // The blocking one, once the arrival's Resume has been over it: the session
    // that asked is on a machine the work has left, so it is locked unanswered
    // rather than left for the human to write into.
    let shut = tokio::time::timeout(WAITING, async {
        loop {
            if b.answering(there, crossed[0], ANSWERED).await == StatusCode::GONE {
                return;
            }

            tokio::time::sleep(LOOKING).await;
        }
    })
    .await;

    assert!(
        shut.is_ok(),
        "the Set the gone session was idling on is locked unanswered here: \
         {idled} on A, {} on B",
        crossed[0],
    );

    // And the Deferred Ask, which the relaunch leaves exactly where it stands:
    // still open on the machine the work is on now, and still the human's to
    // answer there.
    assert!(
        b.answering(there, crossed[1], ANSWERED).await.is_success(),
        "the Deferred Ask is answerable where the work now is",
    );
}

/// **An Attachment opens on the far end**, at a path that device's own sessions
/// are given, with the file's bytes unchanged.
///
/// Rows and bytes both, and the bytes are the half nothing else carries: the row
/// travels in the slice and the file travels beside it, landing in B's own
/// attachments directory under B's own Conversation id. Which is what makes the
/// path one B's sandboxes can mount — a file left under A's id would be a
/// listing in a prompt naming a directory that is not there.
#[tokio::test]
async fn an_attachment_opens_on_the_far_end_with_its_bytes_unchanged() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_draft(&gate, spill.path()).await;

    // Bytes rather than text, because what a human has to hand is a screenshot:
    // a file that crossed as anything but itself is the failure to catch.
    let bytes: Vec<u8> = (0u8..=255).cycle().take(4096).collect();
    let attached = a.attaches(conversation, "burst.bin", &bytes).await;

    assert_eq!(
        a.attached_bytes(conversation, attached).await,
        bytes,
        "it is on the record here to begin with",
    );

    a.grills(conversation).await;
    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    let row = b
        .view(there)
        .await
        .attachments
        .into_iter()
        .find(|attachment| attachment.name == "burst.bin")
        .expect("the file's row crossed with the work");

    assert_eq!(
        row.bytes,
        bytes.len() as i64,
        "the row says how large the file is",
    );

    assert_eq!(
        b.attached_bytes(there, row.id).await,
        bytes,
        "and the file opens over there, byte for byte",
    );

    // And where it opens from: B's own attachments directory, under the
    // Conversation id B gave it, which is the path B's sessions are handed.
    assert!(
        b.attachments_directory(there).join("burst.bin").is_file(),
        "the file is in B's own directory for that Conversation",
    );
}

/// **Nothing of the source machine crosses.** B's Repos, its Agent Profiles, its
/// Members and what it remembers of a Repo's Pairings are exactly what they were
/// before the work arrived.
///
/// Which is the other half of what a slice is: a Conversation's record is a fact
/// about a piece of work, and a Repo, an account and a membership are facts about
/// a machine. A move that carried any of them would be one device quietly
/// registering things on another.
#[tokio::test]
async fn nothing_of_the_source_machine_crosses() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;
    a.asks(conversation, ASKED).await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let repos: Vec<verkstead_render::RepoEntry> = reading(&b.workbench, "/api/ui/repos").await;
    let profiles: Vec<ProfileEntry> = reading(&b.workbench, PROFILES).await;
    let members = store::members(&b.pool).await.unwrap();
    let pairings = store::remembered_pairings(&b.pool, repos[0].id)
        .await
        .unwrap();

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    a.view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await;

    assert_eq!(
        reading::<Vec<verkstead_render::RepoEntry>>(&b.workbench, "/api/ui/repos").await,
        repos,
        "B's Repos are what they were: a registration is a directory on a \
         machine, and nothing about a piece of work registers one",
    );
    assert_eq!(
        reading::<Vec<ProfileEntry>>(&b.workbench, PROFILES).await,
        profiles,
        "and its Agent Profiles, the mirror among them",
    );
    assert_eq!(
        store::members(&b.pool).await.unwrap(),
        members,
        "and the cluster it is part of",
    );
    assert_eq!(
        store::remembered_pairings(&b.pool, repos[0].id)
            .await
            .unwrap(),
        pairings,
        "and what it remembers of that Repo's Pairings, which is a fact about \
         the machine's own last grilling",
    );
}

/// **The branch arrives at the commit it was on**, cut into a Worktree of B's
/// own — and the bundle that carried it was packed against what B said it held
/// rather than against nothing.
///
/// B's repository is a clone of A's here, which is what a cluster holding one
/// repository looks like: the history under the branch is already over there,
/// and what has to cross is the commit the session made. That the bundle leaves
/// the history behind is `transfers::checkouts`'s own test; what this one is
/// about is the branch landing where it stood.
#[tokio::test]
async fn the_branch_arrives_at_the_commit_it_was_on() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move_from_a_clone(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    // What the session committed, which is the one thing B has not got: the
    // base under it came with the clone.
    let worktree = a.worktree(conversation).await;

    std::fs::write(worktree.join("limits.md"), "# Rate limiting\n").unwrap();
    git(&worktree, &["add", "-A"]);
    git(&worktree, &["commit", "-m", "the work so far"]);

    let branch = a.view(conversation).await.branch;
    let at = git_says(&worktree, &["rev-parse", "HEAD"]);

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    let theirs = b.elsewhere.path().join(REPOSITORY);

    assert_eq!(
        git_says(&theirs, &["rev-parse", &format!("refs/heads/{branch}")]),
        at,
        "the branch is at the same commit on both machines",
    );

    // And it is checked out, at a path B named for itself under its own Data
    // Directory — never one of A's, which would mean nothing here.
    let landed = b.worktree(there).await;

    assert!(
        landed.starts_with(b._dir.path().join("worktrees")),
        "B cut the Worktree under its own Data Directory: {}",
        landed.display(),
    );
    assert_ne!(landed, worktree, "and at a path of its own choosing");

    assert_eq!(
        git_says(&landed, &["symbolic-ref", "--short", "HEAD"]),
        branch,
        "the checkout is on the branch the work is on",
    );
    assert_eq!(
        std::fs::read_to_string(landed.join("limits.md")).unwrap(),
        "# Rate limiting\n",
        "and it holds what the session committed",
    );
}

/// **Uncommitted changes on A are uncommitted changes on B**, a changed binary
/// among them, and an untracked file arrives with its bytes unchanged.
///
/// Which is why the patch is a binary one: the Diff a Question Set carries is
/// prose for a human and leaves a changed image out by design, and a move that
/// carried that diff would land a Worktree missing whatever the session had
/// done to a fixture.
#[tokio::test]
async fn the_uncommitted_changes_arrive_as_they_were() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move_from_a_clone(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let worktree = a.worktree(conversation).await;

    // Committed on the branch first, so that there is something for the changes
    // below to be changes *to*.
    std::fs::write(worktree.join("notes.md"), "as committed\n").unwrap();
    std::fs::write(worktree.join("fixture.bin"), COMMITTED_BYTES).unwrap();
    git(&worktree, &["add", "-A"]);
    git(&worktree, &["commit", "-m", "the work so far"]);

    // And left uncommitted on top of it, which is what a turn ends with: an
    // edit, a rewritten binary and a file git has never heard of.
    std::fs::write(worktree.join("notes.md"), "as the session left it\n").unwrap();
    std::fs::write(worktree.join("fixture.bin"), LEFT_BYTES).unwrap();
    std::fs::write(worktree.join("scratch.txt"), "never committed\n").unwrap();

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    let landed = b.worktree(there).await;

    assert_eq!(
        std::fs::read_to_string(landed.join("notes.md")).unwrap(),
        "as the session left it\n",
        "the tracked change crossed",
    );
    assert_eq!(
        std::fs::read(landed.join("fixture.bin")).unwrap(),
        LEFT_BYTES,
        "and so did the changed binary, byte for byte",
    );
    assert_eq!(
        std::fs::read_to_string(landed.join("scratch.txt")).unwrap(),
        "never committed\n",
        "and the untracked file arrived with its bytes unchanged",
    );

    // And they are uncommitted over there too, which is what they were: a move
    // that committed them would be a move that changed the work.
    assert!(
        !git_says(&landed, &["status", "--porcelain"]).is_empty(),
        "the far end's tree has uncommitted changes in it, as this one's had",
    );
}

/// **The whole of a move crosses what an operating system brings with it**, and
/// is read back on the far end: the Timeline, the Worktree and the working
/// changes.
///
/// **Which is what makes the stage demonstrable across two of them.** Three
/// things stand between a move and an OS boundary, and none of them needs a
/// second machine to be wrong:
///
/// - the **line endings**, which is the repositories' own `core.autocrlf` — set
///   here the way a Windows checkout has it on the sending side and the way a
///   Unix one has it on the receiving side, so the tracked change is read out of
///   a tree spelled with CRLF and lands in a tree that keeps LF. That it lands in
///   the receiving tree's convention either way round is
///   `peer::checkouts`'s own pair of tests, which run on both platforms in CI;
///   this is the same crossing with the whole move behind it;
/// - the **path separators**, which is why an untracked file's path is spelled
///   git's way on the wire — a nested one, so that there is a separator in it to
///   get wrong;
/// - and the **Worktree's new path**, which the far end names off its own Data
///   Directory rather than joining anything that arrived onto a directory of its
///   own.
///
/// What is left of a two-OS run after all three is the run itself, which is a
/// human with two machines.
#[tokio::test]
async fn the_move_crosses_what_an_operating_system_brings() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move_from_a_clone(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let worktree = a.worktree(conversation).await;

    // A is the Windows side and B is the Unix one, which between two repositories
    // is the whole of what the difference comes to.
    keeping_endings(&a.repo_path(REPOSITORY), "true", &[&worktree]);
    keeping_endings(&b.repo_path(REPOSITORY), "false", &[]);

    // Committed first, so the changes below are changes *to* something — written
    // with CRLF, the way an editor on that machine writes a file, and stored with
    // LF, the way git stores one.
    std::fs::write(worktree.join("notes.md"), COMMITTED.replace('\n', "\r\n")).unwrap();
    std::fs::write(worktree.join("fixture.bin"), COMMITTED_BYTES).unwrap();
    git(&worktree, &["add", "-A"]);
    git(&worktree, &["commit", "-m", "the work so far"]);

    // And left uncommitted on top of it: the tracked text spelled that machine's
    // way, a rewritten binary, and an untracked file down a path with a separator
    // in it.
    std::fs::write(worktree.join("notes.md"), LEFT.replace('\n', "\r\n")).unwrap();
    std::fs::write(worktree.join("fixture.bin"), LEFT_BYTES).unwrap();
    std::fs::create_dir_all(worktree.join("docs/reading")).unwrap();
    std::fs::write(worktree.join("docs/reading/limits.md"), "not committed\n").unwrap();

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    // The Worktree, read back: B's own path under B's own Data Directory, on the
    // branch the work is on.
    let landed = b.worktree(there).await;

    assert_ne!(landed, worktree, "the Worktree is B's own, at its own path");
    assert!(
        landed.starts_with(b._dir.path()),
        "under B's own Data Directory: {}",
        landed.display(),
    );

    // The working changes, read back: the text in B's convention, the binary byte
    // for byte, and the nested untracked file where its path said.
    assert_eq!(
        std::fs::read_to_string(landed.join("notes.md")).unwrap(),
        LEFT,
        "the tracked change landed in B's own line endings",
    );
    assert_eq!(
        std::fs::read(landed.join("fixture.bin")).unwrap(),
        LEFT_BYTES,
        "and the changed binary landed byte for byte, which is what a binary \
         patch is for",
    );
    assert_eq!(
        std::fs::read_to_string(landed.join("docs/reading/limits.md")).unwrap(),
        "not committed\n",
        "and the untracked file landed down the path it was spelled with",
    );

    // Uncommitted over there too, which is what they were.
    assert!(
        !git_says(&landed, &["status", "--porcelain"]).is_empty(),
        "the far end's tree has uncommitted changes in it, as this one's had",
    );

    // And the Timeline, read back: the session that ran on A, and the Notice
    // saying which machine the work came from.
    let (machine, _) = this_machine();

    let arrived = b
        .view_saying(there, |drawn| {
            drawn
                .timeline
                .iter()
                .any(|event| matches!(event, TimelineEvent::Notice(_)))
        })
        .await;

    assert!(
        arrived
            .timeline
            .iter()
            .any(|event| matches!(event, TimelineEvent::AgentOutput(_))),
        "the session that ran on A is on the far end's Timeline: {:#?}",
        arrived.timeline,
    );

    let came = b.notices(there).await;

    assert!(
        came.iter().any(|notice| notice.contains(&machine)),
        "and a Notice says which machine it came from: {came:#?}",
    );
}

/// **A file the far end's own ignore rules cover is not carried.** A `target/`
/// is the far end's to build, and on the other side of a move it may not even
/// be the same operating system.
/// **A file the far end's own ignore rules cover is not carried.** A `target/`
/// is the far end's to build, and on the other side of a move it may not even
/// be the same operating system.
#[tokio::test]
async fn a_file_the_far_end_ignores_is_not_carried() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move_from_a_clone(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let worktree = a.worktree(conversation).await;

    std::fs::write(worktree.join(".gitignore"), "build/\n").unwrap();
    git(&worktree, &["add", "-A"]);
    git(&worktree, &["commit", "-m", "ignore what is built"]);

    std::fs::create_dir_all(worktree.join("build")).unwrap();
    std::fs::write(worktree.join("build").join("out"), "a build\n").unwrap();
    std::fs::write(worktree.join("kept.txt"), "somebody wrote this\n").unwrap();

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    let landed = b.worktree(there).await;

    assert!(
        !landed.join("build").exists(),
        "the build stayed behind: {}",
        landed.display(),
    );
    assert_eq!(
        std::fs::read_to_string(landed.join("kept.txt")).unwrap(),
        "somebody wrote this\n",
        "and the untracked file beside it did not",
    );
}

/// **A branch a session renamed arrives under the name the checkout is on.**
///
/// Nothing tells Verkstead a session renamed its branch — it is read off the
/// checkout — and the sweep that ordinarily follows one runs only while a
/// session does. A move runs at the end of a turn, which is exactly when that
/// sweep has stopped, so a rename in the last turn is a record one name behind
/// at precisely the moment the work goes.
#[tokio::test]
async fn a_renamed_branch_arrives_under_the_name_the_checkout_is_on() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move_from_a_clone(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let worktree = a.worktree(conversation).await;
    let invented = a.view(conversation).await.branch;

    // What the naming instruction asks of a first session, done the way a
    // session does it: in its own checkout, telling nobody.
    git(&worktree, &["branch", "-m", RENAMED]);

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    assert_ne!(
        invented, RENAMED,
        "the name Verkstead invented is not this one"
    );
    assert_eq!(
        b.view(there).await.branch,
        RENAMED,
        "B's copy is on the name the checkout is on",
    );

    let theirs = b.elsewhere.path().join(REPOSITORY);

    assert_eq!(
        git_says(&theirs, &["rev-parse", &format!("refs/heads/{RENAMED}")]),
        git_says(&worktree, &["rev-parse", "HEAD"]),
        "and the branch over there is that one, at the commit the work is at",
    );
    assert_eq!(
        git_says(
            &b.worktree(there).await,
            &["symbolic-ref", "--short", "HEAD"]
        ),
        RENAMED,
        "and so is the checkout B cut",
    );
}

/// **A read-write Companion arrives with both trees as they were**: both
/// branches at their commits, both sets of uncommitted changes applied.
///
/// Which is the whole of the previous leg said again about another repository.
/// What a read-write Companion is, is a repository a session commits in and
/// leaves uncommitted work in, so nothing about it may be left behind — a
/// Conversation whose own tree crossed and whose Companion's did not is one the
/// work cannot go on in.
#[tokio::test]
async fn a_read_write_companion_arrives_with_both_trees_as_they_were() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) =
        ready_to_move_alongside(&gate, spill.path(), store::CompanionMode::ReadWrite).await;

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let own = a.worktree(conversation).await;
    let beside = a.companion_worktree(conversation, COMPANION).await;

    // A commit in each, and then a change left uncommitted on top of it — which
    // is what a turn ends with in both repositories.
    for (worktree, said) in [(&own, "the server's work\n"), (&beside, "the library's\n")] {
        std::fs::write(worktree.join("limits.md"), said).unwrap();
        git(worktree, &["add", "-A"]);
        git(worktree, &["commit", "-m", "the work so far"]);

        std::fs::write(worktree.join("limits.md"), format!("{said}and more\n")).unwrap();
        std::fs::write(worktree.join("scratch.txt"), "never committed\n").unwrap();
    }

    let branch = a.view(conversation).await.branch;
    let ours = git_says(&own, &["rev-parse", "HEAD"]);
    let theirs = git_says(&beside, &["rev-parse", "HEAD"]);

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    // The branch of each repository on B, at the commit it is at on A.
    for (name, at) in [(REPOSITORY, &ours), (COMPANION, &theirs)] {
        assert_eq!(
            &git_says(
                &b.elsewhere.path().join(name),
                &["rev-parse", &format!("refs/heads/{branch}")],
            ),
            at,
            "{name}'s branch is at the same commit on both machines",
        );
    }

    // And both checkouts, under B's own Data Directory and holding what each
    // session left: what was committed, and what was not.
    let landed = b.worktree(there).await;
    let alongside = b.companion_worktree(there, COMPANION).await;

    assert!(
        alongside.starts_with(b._dir.path().join("worktrees")),
        "B cut the companion's Worktree under its own Data Directory: {}",
        alongside.display(),
    );
    assert_ne!(alongside, beside, "and at a path of its own choosing");
    assert_ne!(
        alongside, landed,
        "and not the one it cut for the work's own"
    );

    for (worktree, said) in [
        (&landed, "the server's work\n"),
        (&alongside, "the library's\n"),
    ] {
        assert_eq!(
            git_says(worktree, &["symbolic-ref", "--short", "HEAD"]),
            branch,
            "the checkout is on the branch the work is on: {}",
            worktree.display(),
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join("limits.md")).unwrap(),
            format!("{said}and more\n"),
            "the tracked change crossed: {}",
            worktree.display(),
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join("scratch.txt")).unwrap(),
            "never committed\n",
            "and so did the untracked file: {}",
            worktree.display(),
        );
    }
}

/// **A read-only Companion arrives detached and bound read-only**, at the commit
/// it was on, with no patch applied to it.
///
/// It has nothing to commit and so nothing uncommitted: carrying a patch to one
/// would be carrying changes a session was never able to make. What says
/// *read-only* on the far end is the Companion's own row, which crossed with the
/// record a leg earlier and is what a sandbox binds the directory by.
#[tokio::test]
async fn a_read_only_companion_arrives_detached_and_bound_read_only() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) =
        ready_to_move_alongside(&gate, spill.path(), store::CompanionMode::ReadOnly).await;

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let beside = a.companion_worktree(conversation, COMPANION).await;
    let at = git_says(&beside, &["rev-parse", "HEAD"]);

    assert!(
        detached(&beside),
        "a read-only companion is detached here, which is what it is to arrive as",
    );

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    let alongside = b.companion_worktree(there, COMPANION).await;

    assert_eq!(
        git_says(&alongside, &["rev-parse", "HEAD"]),
        at,
        "it is at the commit it was at",
    );

    // Detached, which is what says no branch was taken in somebody's repository
    // for a directory nothing will ever be committed in.
    assert!(detached(&alongside), "the companion arrived detached");

    // And nothing was applied to it: the tree is the commit, exactly.
    assert!(
        git_says(&alongside, &["status", "--porcelain"]).is_empty(),
        "no patch was applied to a companion nothing could have written to",
    );

    assert_eq!(
        b.companion_row(there, COMPANION).await.mode,
        store::CompanionMode::ReadOnly,
        "and the row over there binds it read-only, as the row here did",
    );
}

/// **A Companion on the *mirroring* setting arrives under the name a rename gave
/// it.** Its branch is the Conversation's own, followed as that one is renamed,
/// so the act that moves the Conversation's name moves this one with it — and
/// what goes over is the name each checkout is actually on.
#[tokio::test]
async fn a_mirroring_companion_arrives_under_the_name_a_rename_gave_it() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) =
        ready_to_move_alongside(&gate, spill.path(), store::CompanionMode::ReadWrite).await;

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let invented = a.view(conversation).await.branch;
    let own = a.worktree(conversation).await;

    assert!(
        a.companion_row(conversation, COMPANION)
            .await
            .branch
            .is_empty(),
        "the companion is on the mirroring setting, which is what it starts on",
    );

    // What a first session does when the naming instruction asks it to: rename
    // its own branch, telling nobody.
    git(&own, &["branch", "-m", RENAMED]);

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    assert_ne!(invented, RENAMED, "the invented name is not this one");
    assert_eq!(
        b.view(there).await.branch,
        RENAMED,
        "B's copy is on the name the checkout is on",
    );
    assert_eq!(
        git_says(
            &b.companion_worktree(there, COMPANION).await,
            &["symbolic-ref", "--short", "HEAD"],
        ),
        RENAMED,
        "and so is the companion, the mirror rule resolving to the new name",
    );
    assert_eq!(
        git_says(
            &b.elsewhere.path().join(COMPANION),
            &["rev-parse", &format!("refs/heads/{RENAMED}")],
        ),
        git_says(
            &a.companion_worktree(conversation, COMPANION).await,
            &["rev-parse", "HEAD"],
        ),
        "at the commit the companion's work is at",
    );
}

/// **Ignored files in a Companion stay behind**, as they do in the Conversation's
/// own repository: a build is the far end's to make, and on the other side of a
/// move it may not even be the same operating system.
#[tokio::test]
async fn an_ignored_file_in_a_companion_stays_behind() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) =
        ready_to_move_alongside(&gate, spill.path(), store::CompanionMode::ReadWrite).await;

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let beside = a.companion_worktree(conversation, COMPANION).await;

    std::fs::write(beside.join(".gitignore"), "build/\n").unwrap();
    git(&beside, &["add", "-A"]);
    git(&beside, &["commit", "-m", "ignore what is built"]);

    std::fs::create_dir_all(beside.join("build")).unwrap();
    std::fs::write(beside.join("build").join("out"), "a build\n").unwrap();
    std::fs::write(beside.join("kept.txt"), "somebody wrote this\n").unwrap();

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    let alongside = b.companion_worktree(there, COMPANION).await;

    assert!(
        !alongside.join("build").exists(),
        "the companion's build stayed behind: {}",
        alongside.display(),
    );
    assert_eq!(
        std::fs::read_to_string(alongside.join("kept.txt")).unwrap(),
        "somebody wrote this\n",
        "and the untracked file beside it did not",
    );
}

/// The fixture as it was committed, and as the session left it — bytes rather
/// than text, which is the whole point of them: a patch that was not a binary
/// one would carry neither.
const COMMITTED_BYTES: &[u8] = &[0x00, 0x01, 0x02, 0x03];
const LEFT_BYTES: &[u8] = &[0xff, 0xfe, 0x00, 0x7f, 0x80];

/// And the text file beside it, written with `\n` here and spelled each
/// machine's own way by whoever writes it into a tree — which is what a
/// line-ending convention is.
const COMMITTED: &str = "one\ntwo\n";
const LEFT: &str = "one\ntwo\nthree\n";

/// And what a session renames its branch to, which is nothing Verkstead would
/// have invented.
const RENAMED: &str = "rate-limiting";

/// The login a session on A refreshes inside itself, which is what a harness does
/// to the account it is running as — see [`refreshes_its_login_at`].
const REFRESHED: &str = "sk-ant-oat01-refreshed-mid-turn";

/// **A running grilling on the source continues as a re-primed grilling on the
/// far end**, started by that device's own Resume.
///
/// Which is the whole of what an arrival is. Resume is the one standing way in —
/// it asks what *ought* to be running now, from the lifecycle the Conversation is
/// in and what the branch has written — and a Conversation that has just landed
/// poses exactly that question. What it gives is a fresh session primed from the
/// record: Verkstead's own Resume rather than the harness's, which is stage 10.
///
/// Read off the session B started rather than off a flag: the prompt it was
/// launched on carries the Brief and what the grilling has already settled, and
/// neither of those was on this machine a moment ago.
#[tokio::test]
async fn a_running_grilling_continues_as_a_re_primed_grilling_over_there() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    // A turn genuinely in flight, with something settled in it: the digest a
    // relaunch is primed with is made of Answers, so an Answer is what makes the
    // prompt on the far end more than the Brief.
    a.printed(conversation, "grilling").await;

    let set = a.asks(conversation, ASKED).await;
    a.answers(conversation, set, ANSWERED).await;

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let crossed = a.sessions_on(conversation).await;

    assert_eq!(
        crossed.len(),
        1,
        "one session ran here, and it is the one that crosses",
    );

    // The turn ends, and the move follows it.
    std::fs::write(&gate, "go").unwrap();

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    // B started a session of its own, which is the one thing nothing but its
    // Resume could have done: nothing on A asked for it, and B had never heard of
    // this work.
    let said = b.latest_capture_saying(there, "re-primed").await;

    assert_eq!(
        b.sessions_on(there).await.len(),
        2,
        "two sessions on the far end's Timeline: the one that crossed, and the \
         one this device started — it printed {said:?}",
    );

    assert!(
        said.contains("Rate limiting"),
        "the session was primed with the Brief: {said:?}",
    );
    assert!(
        said.contains("In Redis"),
        "and with what the grilling had already settled, which is the digest a \
         relaunch carries: {said:?}",
    );

    assert_eq!(
        b.view(there).await.state,
        Lifecycle::Grilling,
        "and it is still a grilling: what moved is where the work is being done \
         rather than how far it has got",
    );
}

/// **Each end's Timeline says what happened to it**: the far end's that the work
/// came from that machine, and the source's that it went to the other one.
///
/// So the record reads as one story across the two databases rather than as a
/// Conversation that appeared on one from nowhere and stopped on the other for no
/// reason. By the name the human gave each machine, which is what every sentence
/// about a member is written with.
#[tokio::test]
async fn each_end_says_where_the_work_came_from_and_went_to() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");
    let (a, b, _holding, conversation) = ready_to_move(&gate, spill.path()).await;

    a.printed(conversation, "grilling").await;
    std::fs::write(&gate, "go").unwrap();
    a.view_saying(conversation, |drawn| !drawn.working).await;

    let (machine, _) = this_machine();

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    let there = a
        .view_saying(conversation, |drawn| drawn.transferred.is_some())
        .await
        .transferred
        .expect("the mark the move wrote")
        .id;

    // The source's, which is on the copy it kept: the page redirects off a
    // tombstone rather than drawing it, so what this is written on is the record
    // — which is what a Share of it reads, and what a transfer back replaces.
    let here = a.notices(conversation).await;
    let went = here
        .last()
        .expect("the move puts a Notice on the copy it left behind");

    assert!(
        went.contains(B_MACHINE),
        "the source's Notice names the machine the work went to: {went}",
    );

    // And the far end's, which is the first thing written on its copy that did
    // not cross with the record.
    let arrived = b
        .view_saying(there, |drawn| {
            drawn
                .timeline
                .iter()
                .any(|event| matches!(event, TimelineEvent::Notice(_)))
        })
        .await;

    let came = arrived
        .timeline
        .iter()
        .filter_map(|event| match event {
            TimelineEvent::Notice(notice) => Some(notice.html.clone()),
            _ => None,
        })
        .next()
        .expect("the arrival puts a Notice on the copy that landed");

    assert!(
        came.contains(&machine),
        "and the far end's names the machine the work came from: {came}",
    );
}

/// **The account goes home before the work leaves**, and the session the far end
/// starts runs under it.
///
/// A session away from home keeps a mirror of its account and writes the login it
/// now holds back to the device that account is at home on as it ends — stage
/// 08's machinery, unchanged. The move runs at the turn's end, which is exactly
/// when that write-back happens, so the ordering is the thing this is about: the
/// account is home *before* the slice and the bundle leave, or the far end would
/// launch under a login the source had not finished returning.
///
/// The work runs on A under an account at home on **B**, which is the way round
/// that makes the question askable at all: the write-back's destination is the
/// machine the work is about to move to, so a move that overtook it would be
/// visible on the far end as a session running signed in as somebody else.
///
/// Read at the moment B's **row** lands, which is the first of the three legs: the
/// slice and the bundle come after it, so a login already home by then is one
/// that was written before any of the work left.
///
/// **The login stands for both halves of the write-back.** A session's ending
/// puts the memory store back first and the login after it — the account keeping
/// its login inside the very data directory the memory sync carries whole, so the
/// login has to have the last word over it. A login that is home is therefore a
/// memory that is home; the memory sync's own end-to-end proof is
/// `tests/accounts.rs`.
#[tokio::test]
async fn the_account_is_home_before_the_work_leaves_and_the_far_end_runs_under_it() {
    let spill = tempfile::tempdir().unwrap();
    let gate = spill.path().join("go");

    let a = Verkstead::running(A, &refreshes_its_login_at(&gate, REFRESHED), spill.path()).await;
    let b = Verkstead::running(B, &re_primed(), spill.path()).await;

    a.linked_to(&b.device, B_MACHINE, B_OS, vec![b.at()]).await;

    let (machine, os) = this_machine();
    b.linked_to(&a.device, &machine, &os, vec![a.at()]).await;

    a.repo().await;
    b.repo().await;

    // The account is B's, and what A holds is a mirror of it — the other way
    // round from every other test here, which is the point.
    b.account().await;

    let _holding = a.holding();
    let mirror = a.profile_called(ACCOUNT).await;

    let conversation = a.drafting_under(mirror.id).await;
    a.grills(conversation).await;

    // The session is up, has been lent B's account, and has refreshed the login
    // inside it — which is the one file of an account a harness really changes.
    a.printed(conversation, "refreshed the login").await;

    assert!(
        !b.login_at_home(ACCOUNT).contains(REFRESHED),
        "nothing has been written home while the turn is still running",
    );

    assert_eq!(a.transfers(conversation, B).await, "\"Transferring\"");

    std::fs::write(&gate, "go").unwrap();

    // The row is the first thing to cross, so this is the earliest moment the
    // work can be said to have left at all.
    let deadline = Instant::now() + WAITING;

    let landed = loop {
        let rows = b.own_rows().await;

        if let Some(row) = rows.first() {
            break row.id;
        }

        assert!(
            Instant::now() < deadline,
            "nothing ever arrived on the far end",
        );

        tokio::time::sleep(LOOKING).await;
    };

    let held = b.login_at_home(ACCOUNT);

    assert!(
        held.contains(REFRESHED),
        "the login the session refreshed was home before the work left: the \
         account holds {held:?}",
    );

    // And the session B's own Resume starts is running as that account, with the
    // login this turn left in it.
    let said = b.latest_capture_saying(landed, "login=").await;

    assert!(
        said.contains(REFRESHED),
        "the session on the far end runs under the account the source had, with \
         the login it was left holding: {said:?}",
    );
}

/// The Set every test here asks, which asks one thing so that there is an Answer
/// to give it.
const ASKED: &str = r#"
title: Where the counter lives
preface: The API has no rate limiting at all.
questions:
  - label: Q1
    text: Where should the counter live?
    options:
      - n: 1
        text: In Redis
        recommended: true
      - n: 2
        text: In the process
"#;

/// And the Answer given on the far end, with a word about why so that what the
/// wait is handed can be told apart from the Set it answers.
const ANSWERED: &str = r#"
answers:
  - label: Q1
    selected: 1
    free_text: shared between instances
"#;

/// The Timeline as the record rather than as a set of row numbers.
///
/// Every id taken out, because the ids are the whole of what two copies of one
/// Conversation are allowed to differ by — and the standing with them, which is
/// a reading of a moment: whether a Set is being waited on is about the session
/// running now rather than about the record.
fn cards(timeline: &[TimelineEvent]) -> serde_json::Value {
    let mut drawn = serde_json::to_value(timeline).unwrap();

    plainly(&mut drawn);

    drawn
}

/// Every `id`, `set_id` and `standing` taken out of a drawn Timeline, however
/// deep it is.
fn plainly(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(fields) => {
            for named in ["id", "set_id", "standing"] {
                fields.remove(named);
            }

            for (_, inside) in fields.iter_mut() {
                plainly(inside);
            }
        }

        serde_json::Value::Array(items) => {
            for inside in items {
                plainly(inside);
            }
        }

        _ => {}
    }
}

/// Every session name one device's store holds, in the order of the Events they
/// hang off.
async fn session_names(pool: &SqlitePool) -> Vec<(i64, String)> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT session_id FROM session_names
         JOIN timeline_events ON timeline_events.id = session_names.event_id
         ORDER BY timeline_events.id",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    rows.into_iter()
        .enumerate()
        .map(|(at, (name,))| (at as i64, name))
        .collect()
}

/// And every Transcript line, the same way.
async fn transcripts(pool: &SqlitePool) -> Vec<String> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT line FROM transcript_lines
         JOIN timeline_events ON timeline_events.id = transcript_lines.event_id
         ORDER BY timeline_events.id, transcript_lines.seq",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    rows.into_iter().map(|(line,)| line).collect()
}
