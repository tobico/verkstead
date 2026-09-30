//! **The preflight**: what a device lacks before a Conversation can be moved
//! onto it (ADR-0020, *Transfer*).
//!
//! **Two Verksteads, real repositories, a real dial and a real `PATH`.**
//! `tests/matching.rs` is the rule that settles which of a member's Repos is
//! this repository; this is the reading built on it, and everything beside it
//! is asked of the far end over the Peer Listener behind the Member Gate. So
//! nothing here hands A a fact about B by the back door: every repository is a
//! directory `git init` really ran in with the remote it is supposed to have,
//! and every harness is a program really on a `PATH` — or really not.
//!
//! **What every assertion reads is the endpoint the dialog reads**:
//! `/api/ui/conversations/{id}/preflight/{device}` on the device the
//! Conversation is on, which is where the Repo match has to run — the far end
//! sends its registry and the end that is going to act on the answer applies the
//! rule.
//!
//! **And the pair worth keeping apart is *no match* and *unreachable*.** One
//! sends the human to **Open repo** on a machine that may already have the
//! repository; the other sends them to the machine. A device that answered
//! nothing is the whole of its own preflight here, which is the assertion that
//! says so.
//!
//! **Unix, with `tests/mirroring.rs` and `tests/accounts.rs`**, and for their
//! reason: what a device here is judged on is a `PATH` of this suite's own
//! making, with a shell stub for each harness it is to have — a `#!/bin/sh`
//! script and a mode, which is not what *installed* means on Windows.

#![cfg(unix)]

use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{Lacking, PairingRole, Preflight, ProfileEntry};
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Environment, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::{Routers, open_database, routers_answering_devices_telling, store};
use verkstead_store::{Linking, record_member};

/// The device the work is on, and where every assertion is made: the preflight
/// is read on the end that holds the Conversation.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the device it would be moved onto, which is the one being asked about.
const B: &str = "0011223344556677889900aabbccddee";

/// What B is written down as on A: the name the human gave the machine, which is
/// what every finding has to be said against rather than sixteen bytes of hex.
const B_MACHINE: &str = "the-laptop";
const B_OS: &str = "macOS 15.1";

/// The Profiles the pairings are picked out of, and the model each lists.
const PROFILES: &str = "/api/ui/profiles";
const MODEL: &str = "claude-opus-5";

/// Every harness a device in this suite normally has, named the way a session
/// launches one.
const EVERY_HARNESS: &[&str] = &["claude", "codex", "grok", "opencode"];

/// How long one address has to answer here, rather than the two seconds a
/// running server gives one.
const PATIENCE: std::time::Duration = std::time::Duration::from_millis(300);

/// One Verkstead: its store, its identity, its workbench router and its Peer
/// Listener up behind the Member Gate.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    workbench: Router,
    address: SocketAddr,

    /// Held for the length of the test: the identity, the database, the
    /// accounts and every repository registered on it live in it.
    dir: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead with every harness on the `PATH` a session here would
    /// search, which is the ordinary machine.
    async fn answering(id: &str) -> Verkstead {
        Verkstead::standing(id, EVERY_HARNESS).await
    }

    /// And one with none at all, which is the machine a Pairing cannot be
    /// launched on.
    async fn answering_with_no_harness(id: &str) -> Verkstead {
        Verkstead::standing(id, &[]).await
    }

    async fn standing(id: &str, harnesses: &[&str]) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let machine = probing(dir.path(), harnesses);

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
        } = routers_answering_devices_telling(pool.clone(), cluster, nudges.clone(), machine);

        tokio::spawn(listener.serving(peer::router(
            device.clone(),
            reading,
            members,
            Joins::none(),
            nudges,
            over_the_link,
        )));

        Verkstead {
            device,
            pool,
            workbench,
            address,
            dir,
        }
    }

    /// The one address it advertises.
    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
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

    /// A repository on this machine, registered as a Repo — with the `origin`
    /// remote `origin` names, or none at all.
    ///
    /// Really made and really registered: the origin the match compares is the
    /// one git answers with, so a test that wrote a URL into a row would be
    /// testing nothing this does.
    async fn repo(&self, name: &str, origin: Option<&str>) -> store::Repo {
        let path = repository(self.dir.path().join("repos").join(name));

        if let Some(url) = origin {
            git(&path, &["remote", "add", "origin", url]);
        }

        let repo = store::register_repo(&self.pool, &path, name, "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet");

        store::registered_repo(&self.pool, repo.id)
            .await
            .unwrap()
            .expect("the Repo that was just registered")
    }

    /// An account on this machine for a Profile to name: the pair Claude Code
    /// keeps one as, really made, because saving a Profile is refused where the
    /// paths are not there.
    async fn saves(&self, name: &str) -> i64 {
        let under = self.dir.path().join("accounts").join(name);
        let claude_dir = under.join(".claude");
        let config_file = under.join(".claude.json");

        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(&config_file, "{}\n").unwrap();

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

        let profiles: Vec<ProfileEntry> = reading(&self.workbench, PROFILES).await;

        profiles
            .iter()
            .find(|profile| profile.name.as_deref() == Some(name))
            .expect("the Profile that was just saved")
            .id
    }

    /// A Conversation on `repo`, with all three of its pickers filled in the
    /// way the composer fills them.
    async fn conversation(&self, repo: &store::Repo, profile: i64) -> i64 {
        let id = store::start_conversation(&self.pool, repo.id, "amber-kestrel", self.device.id())
            .await
            .unwrap()
            .expect("the Repo is registered");

        let pairing = serde_json::json!({ "profile_id": profile, "model": MODEL }).to_string();
        let role =
            serde_json::json!({ "pairing": { "profile_id": profile, "model": MODEL } }).to_string();

        for (path, saying) in [
            ("grilling-pairing", &pairing),
            ("implementation-pairing", &pairing),
            ("review-pairing", &role),
        ] {
            let said = press(
                &self.workbench,
                &format!("/api/ui/conversations/{id}/{path}"),
                Some(saying),
            )
            .await;

            assert_eq!(said, "\"Chosen\"", "picking {path}");
        }

        id
    }

    /// What `device` lacks before that Conversation could be moved onto it, as
    /// the dialog reads it.
    async fn preflight(&self, conversation: i64, device: &str) -> Preflight {
        reading(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/preflight/{device}"),
        )
        .await
    }
}

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422)
}

/// The machine a device here is judged on: one directory on the `PATH` a session
/// would search, holding a program for each of `harnesses`.
///
/// **Stated rather than read off the runner**, for the reason the mirrors' suite
/// states its own: whether a harness is on a device is exactly what one of these
/// tests is about, so what this box happens to have `claude` installed as must
/// not be what decides it.
fn probing(dir: &Path, harnesses: &[&str]) -> Machine {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();

    for harness in harnesses {
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

/// A and B, linked both ways — A holding the work, B the machine it would go to.
///
/// Both ways because a link is both ways: A dials B pinned on B's fingerprint,
/// and B admits A because A's certificate is one it holds a membership for.
async fn linked_up() -> (Verkstead, Verkstead) {
    link(Verkstead::answering(A).await, Verkstead::answering(B).await).await
}

/// The same link made over whichever B the test wants — the one with no harness
/// on it, say.
async fn link(a: Verkstead, b: Verkstead) -> (Verkstead, Verkstead) {
    a.linked_to(&b.device, B_MACHINE, B_OS, vec![b.at()]).await;

    let (machine, os) = this_machine();
    b.linked_to(&a.device, &machine, &os, vec![a.at()]).await;

    (a, b)
}

/// What a device says about the machine it is on, read the way the server reads
/// it.
fn this_machine() -> (String, String) {
    (
        platform::hostname(),
        platform::os_word(Platform::Linux, None),
    )
}

/// A git repository at `path`, with one commit on `main` so it has a branch to
/// call its default.
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

/// The ordinary case: the far end has the repository and the harness, and there
/// is nothing in the way at all.
#[tokio::test]
async fn a_device_that_has_it_all_lacks_nothing() {
    let (a, b) = linked_up().await;

    let origin = "https://github.com/you/widgets.git";

    // The same repository at two paths under two names, which is what one
    // repository across two machines looks like.
    b.repo("widgets-clone", Some(origin)).await;
    let here = a.repo("widgets", Some(origin)).await;

    let profile = a.saves("work").await;
    let conversation = a.conversation(&here, profile).await;

    let found = a.preflight(conversation, B).await;

    assert_eq!(found.device, B_MACHINE, "the machine by the name given it");
    assert_eq!(
        found.lacks,
        Vec::new(),
        "nothing is in the way of moving the work there",
    );
}

/// A repository the far end has no Repo for is named, and named as the
/// Conversation's own rather than as a companion's.
#[tokio::test]
async fn a_repo_with_no_match_is_named() {
    let (a, b) = linked_up().await;

    // B has a repository of the same name and another origin, which is two
    // repositories rather than one — see `tests/matching.rs`.
    b.repo("widgets", Some("https://github.com/someone/widgets.git"))
        .await;

    let here = a
        .repo("widgets", Some("https://github.com/you/widgets.git"))
        .await;

    let profile = a.saves("work").await;
    let conversation = a.conversation(&here, profile).await;

    assert_eq!(
        a.preflight(conversation, B).await.lacks,
        vec![Lacking::Repo {
            name: "widgets".to_owned(),
            companion: false,
        }],
    );
}

/// And a Companion with nowhere to land is named as one: it travels the same way
/// the branch does, and it is a different thing to go and put right.
#[tokio::test]
async fn a_companion_with_no_match_is_named_as_a_companion() {
    let (a, b) = linked_up().await;

    let origin = "https://github.com/you/widgets.git";

    b.repo("widgets-clone", Some(origin)).await;
    let here = a.repo("widgets", Some(origin)).await;

    // And a second repository beside it that B has never heard of.
    let beside = a
        .repo("askance", Some("https://github.com/you/askance.git"))
        .await;

    let profile = a.saves("work").await;
    let conversation = a.conversation(&here, profile).await;

    store::add_companion(&a.pool, conversation, beside.id)
        .await
        .unwrap();

    assert_eq!(
        a.preflight(conversation, B).await.lacks,
        vec![Lacking::Repo {
            name: "askance".to_owned(),
            companion: true,
        }],
    );
}

/// A harness the far end has not got is named against each Pairing that wants
/// it, with the Profile those Pairings run under.
#[tokio::test]
async fn a_harness_the_far_end_has_not_got_is_named_with_its_pairings() {
    let (a, b) = link(
        Verkstead::answering(A).await,
        Verkstead::answering_with_no_harness(B).await,
    )
    .await;

    let origin = "https://github.com/you/widgets.git";

    b.repo("widgets-clone", Some(origin)).await;
    let here = a.repo("widgets", Some(origin)).await;

    let profile = a.saves("work").await;
    let conversation = a.conversation(&here, profile).await;

    let harness = |role| Lacking::Harness {
        role,
        profile: Some("work".to_owned()),
        agent_type: verkstead_render::AgentType::Claude,
    };

    assert_eq!(
        a.preflight(conversation, B).await.lacks,
        vec![
            harness(PairingRole::Grilling),
            harness(PairingRole::Implementation),
            harness(PairingRole::Review),
        ],
        "the repository is there and the harness the three Pairings run is not",
    );
}

/// And a device that answered nothing is the whole of its own preflight, named
/// — never *no match*, which would send the human to open a repository on a
/// machine that may already have it.
#[tokio::test]
async fn a_device_that_is_not_answering_is_unreachable_and_nothing_else() {
    let a = Verkstead::answering(A).await;

    // A device that is a member and is not there: the identity is real, so the
    // membership row is a real one, and nothing is listening at the address.
    let away = tempfile::tempdir().unwrap();
    let asleep = Device::stated(away.path(), B).unwrap();

    a.linked_to(&asleep, B_MACHINE, B_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    let here = a
        .repo("widgets", Some("https://github.com/you/widgets.git"))
        .await;

    let profile = a.saves("work").await;
    let conversation = a.conversation(&here, profile).await;

    let found = a.preflight(conversation, B).await;

    assert_eq!(found.device, B_MACHINE, "and the machine is named");
    assert_eq!(
        found.lacks,
        vec![Lacking::Unreachable],
        "a machine that said nothing has failed none of the questions under it",
    );
}
