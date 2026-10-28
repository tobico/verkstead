//! **Agent Profiles as records**: every device holds the one record of every
//! Profile and pulls a member's on its news (ADR-0028), so every Profiles
//! section and every pairing picker lists the cluster's Profiles, each with or
//! without a binding of this device's.
//!
//! **Two Verksteads, a real dial and a real stream.** Nothing here hands B a
//! record of A's by the back door: B pulls A's records off the route every
//! device serves them on, over the Peer Listener, on the Nudge stream it holds
//! to A — and what every assertion reads is a device's own `/api/ui/profiles`,
//! the one an open Profiles section and every picker ask for.
//!
//! **And an edit or a removal is a local write on whichever device it is
//! pressed**, which reaches the others the same way: so a press on B is
//! asserted on A, and a third device learns of it off whoever holds it.
//!
//! **Unix, with `tests/accounts.rs` and `tests/news.rs`**, and for the reason
//! those two are: what a device here is judged on is a `PATH` of this suite's
//! own making, with a shell stub for each harness it is to have — a Profile
//! not bound here whose harness is not on the box reads broken, so the box this
//! runs on must not be what decides. That stub is a `#!/bin/sh` script and a
//! mode, which is not what *installed* means on Windows.

#![cfg(unix)]

use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{ConversationView, DevicesView, ProfileEntry};
use verkstead_schema::Nudge;
use verkstead_server::device::reading::{RUNNING, Reading};
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Environment, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::{Routers, open_database, routers_keeping_answering_devices_telling, store};
use verkstead_store::{Linking, forget_member, record_member};

/// The device whose Profiles are made first.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the device they reach, which is where most assertions are made.
const B: &str = "0011223344556677889900aabbccddee";

/// And a third, linked to A alone: what reaches it from B reaches it through A.
const C: &str = "ff11223344556677889900aabbccddee";

/// What A is written down as on B: the name and the OS word B holds against A's
/// row, which is what a line on its Devices row names it by.
///
/// Stated rather than read off a machine, because they are the membership row's
/// — a member's name and OS are what it last said about itself, at an exchange
/// or when its stream was taken up — and A says them of itself to match. And
/// deliberately not
/// this box's own words: every device in this suite is one machine, so a row
/// that took B's hostname for A's would read right for the wrong reason.
const A_MACHINE: &str = "the-workstation";
const A_OS: &str = "macOS 15.1";

/// The Profiles section and every pairing picker, which is the path every
/// assertion here reads.
const PROFILES: &str = "/api/ui/profiles";

/// Where a member pulls a device's Profile records and their bindings: the
/// records route, by the two kinds' names, which both begin so.
const RECORDS: &str = "/api/peer/v1/records/profile";

/// The model every Profile in this suite lists.
const MODEL: &str = "claude-opus-5";

/// How long one address has to answer here, rather than the two seconds a
/// running server gives one.
const PATIENCE: std::time::Duration = std::time::Duration::from_millis(300);

/// How long a test will wait for the mirrors to say what it is waiting for.
/// Generous, because it is only ever paid in full when the assertion is about to
/// fail — what it waits on is a Nudge crossing a link and a list read back over
/// one, both on the loopback.
const WAITING: std::time::Duration = std::time::Duration::from_secs(10);

/// And how often it looks while it waits.
const LOOKING: std::time::Duration = std::time::Duration::from_millis(50);

/// And how long a test waits to be sure nothing is going to change.
const SETTLING: std::time::Duration = std::time::Duration::from_secs(2);

/// One Verkstead: its store, its identity, the Nudge channel its own pages and
/// its members both read, and both of the routers standing over its one state.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    nudges: Nudges,
    cluster: Devices,
    workbench: Router,
    address: SocketAddr,

    /// What it says of itself, which a suite moves onto another release.
    reading: Reading,

    /// And whether it answers its Profile records with something no Verkstead
    /// can read, which is what a member a release apart looks like — see
    /// [`Verkstead::garbles`].
    garbling: Arc<AtomicBool>,

    /// And whether it answers the records route for Profiles at all, which a
    /// member too old to serve it does not — see [`Verkstead::too_old`].
    refusing: Arc<AtomicBool>,

    /// Held for the length of the test: the identity, the database and the
    /// accounts the Profiles here name all live in it.
    dir: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead with a store of its own, an identity in it, and its Peer
    /// Listener up — both routers over the one state, the peer half behind the
    /// Member Gate.
    async fn answering(id: &str) -> Verkstead {
        Verkstead::standing(id, EVERY_HARNESS).await
    }

    /// And one this suite says has no harness on it at all, which is the one
    /// thing about a Profile not bound here that is a fact about the device
    /// drawing it rather than about the account.
    async fn answering_with_no_harness(id: &str) -> Verkstead {
        Verkstead::standing(id, &[]).await
    }

    /// Both of them, which differ in what the onboarding probe finds on the
    /// `PATH` a session here would search.
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

        // A says of itself what every row for it here says, so that a member
        // reading its identity again on taking up its stream finds that row as
        // it stands — see [`A_MACHINE`].
        let reading = if id == A {
            reading.calling_itself(A_MACHINE, A_OS)
        } else {
            reading
        };

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
        } = routers_keeping_answering_devices_telling(
            pool.clone(),
            dir.path().join("data"),
            cluster.clone(),
            nudges.clone(),
            machine,
        );

        let garbling = Arc::new(AtomicBool::new(false));
        let refusing = Arc::new(AtomicBool::new(false));

        // What a member pulls of this device's Profiles, answered unreadably
        // while the test says so — or not answered at all, the way a release
        // from before the records route answers it.
        let over_the_link = over_the_link.layer(axum::middleware::from_fn({
            let garbling = garbling.clone();
            let refusing = refusing.clone();

            move |asked: Request<Body>, next: axum::middleware::Next| {
                let path = asked.uri().path();
                let profiles = path.starts_with(RECORDS);
                let garbled = garbling.load(Ordering::SeqCst) && profiles;
                let refused = refusing.load(Ordering::SeqCst) && profiles;

                async move {
                    match (garbled, refused) {
                        (true, _) => {
                            axum::response::IntoResponse::into_response("{\"not\": \"a list\"}")
                        }
                        (_, true) => {
                            axum::response::IntoResponse::into_response(StatusCode::NOT_FOUND)
                        }
                        _ => next.run(asked).await,
                    }
                }
            }
        }));

        tokio::spawn(listener.serving(peer::router(
            device.clone(),
            reading.clone(),
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
            reading,
            garbling,
            refusing,
            dir,
        }
    }

    /// Answer the records route for Profiles with a `404`, the way a release
    /// from before it does, or stop.
    fn too_old(&self, refusing: bool) {
        self.refusing.store(refusing, Ordering::SeqCst);
    }

    /// Answer its Profile records with something no Verkstead can read, or
    /// stop.
    fn garbles(&self, garbling: bool) {
        self.garbling.store(garbling, Ordering::SeqCst);
    }

    /// The line this device's Devices section draws under `member`'s row, once
    /// it says what the test is waiting for, or a panic saying what it did say.
    async fn trouble_on(&self, member: &str, what: Option<&str>) {
        let mut last = None;

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                let view: DevicesView = reading(&self.workbench, "/api/ui/devices").await;

                last = view
                    .members
                    .into_iter()
                    .find(|row| row.identity.device == member)
                    .expect("the member has a row")
                    .trouble;

                if last.as_deref() == what {
                    return;
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("the row never read {what:?}: it read {last:?}"));
    }

    /// Hold a Nudge stream to every member of this device's cluster, which is
    /// what a running server spawns at the start.
    ///
    /// The handle is kept by the caller so the streams are let go of when the
    /// test ends rather than left dialling a listener that has gone.
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

    /// Write `other` down as one of this device's members, at `addresses`, under
    /// the name and OS word it is to be shown as.
    async fn linked_to(&self, other: &Device, name: &str, os: &str, addresses: Vec<String>) {
        self.linked_on(other, name, os, addresses, RUNNING).await;
    }

    /// The same, recording the member as running `version`.
    async fn linked_on(
        &self,
        other: &Device,
        name: &str,
        os: &str,
        addresses: Vec<String>,
        version: &str,
    ) {
        record_member(
            &self.pool,
            &Linking {
                device: other.id().to_owned(),
                name: name.to_owned(),
                os: os.to_owned(),
                addresses,
                fingerprint: other.fingerprint().to_owned(),
                kept_by: None,
                version: Some(version.to_owned()),
                install: None,
            },
        )
        .await
        .unwrap();
    }

    /// An account on this machine for a Profile to name: the pair Claude Code
    /// keeps one as, really made, because saving a Profile is refused where the
    /// paths are not there.
    ///
    /// **Logged in**, which is what an account somebody uses is: the login is
    /// the one file of it another device can be lent.
    fn account(&self, name: &str) -> (PathBuf, PathBuf) {
        let (claude_dir, config_file) = self.account_nobody_is_logged_in_to(name);

        std::fs::write(
            claude_dir.join(".credentials.json"),
            "{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-work\"}}",
        )
        .unwrap();

        (claude_dir, config_file)
    }

    /// And the same account with no login file in it.
    fn account_nobody_is_logged_in_to(&self, name: &str) -> (PathBuf, PathBuf) {
        let under = self.dir.path().join("accounts").join(name);
        let claude_dir = under.join(".claude");
        let config_file = under.join(".claude.json");

        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(&config_file, "{}\n").unwrap();

        (claude_dir, config_file)
    }

    /// A Profile saved on this device, the way the form saves one.
    async fn saves(&self, name: &str) -> String {
        let account = self.account(name);

        self.saving(name, account).await
    }

    /// Both of them: the form's own press, over whichever account was made.
    async fn saving(&self, name: &str, (claude_dir, config_file): (PathBuf, PathBuf)) -> String {
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

        self.profiles()
            .await
            .into_iter()
            .find(|profile| profile.name.as_deref() == Some(name))
            .expect("the Profile that was just saved is on the list")
            .id
    }

    /// And rewritten, whole, which is what the form does to one that is already
    /// there.
    async fn renames(&self, id: &str, to: &str) {
        let (claude_dir, config_file) = self.account(to);

        let said = press(
            &self.workbench,
            &format!("{PROFILES}/{id}"),
            Some(
                &serde_json::json!({
                    "name": to,
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

        assert_eq!(said, "\"Saved\"", "renaming {id} to {to}");
    }

    /// And taken away.
    async fn removes(&self, id: &str) {
        let said = press(&self.workbench, &format!("{PROFILES}/{id}/delete"), None).await;

        assert_eq!(said, "\"Removed\"", "removing {id}");
    }

    /// And one nobody named, which is the account nearly every installation
    /// holds: what the second of the two uniqueness rules is about.
    async fn saves_unnamed(&self) {
        let (claude_dir, config_file) = self.account("default");

        let said = press(
            &self.workbench,
            PROFILES,
            Some(
                &serde_json::json!({
                    "name": null,
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

        assert_eq!(said, "\"Saved\"", "saving the account nobody named");
    }

    /// The form over one row of this device's list, saved under another name.
    ///
    /// **The fields the pane really sends**: the account, the models and the
    /// memory switch as the row itself carries them, with the name changed — which
    /// is what the human does to a Profile they are renaming, and is the same form
    /// whether or not the row is bound here. The paths on a row not bound here
    /// are another device's, which the save leaves alone.
    ///
    /// The status is given back with the answer.
    async fn saves_over(&self, row: &ProfileEntry, name: Option<&str>) -> (StatusCode, String) {
        pressing(
            &self.workbench,
            &format!("{PROFILES}/{}", row.id),
            Some(
                &serde_json::json!({
                    "name": name,
                    "account": row.account,
                    "models": row.models,
                    "memory": row.memory,
                })
                .to_string(),
            ),
        )
        .await
    }

    /// And the removal, with the status left to be read for the same reason.
    async fn pressing_removal(&self, id: &str) -> (StatusCode, String) {
        pressing(&self.workbench, &format!("{PROFILES}/{id}/delete"), None).await
    }

    /// The Profiles as this device's own browser reads them.
    async fn profiles(&self) -> Vec<ProfileEntry> {
        reading(&self.workbench, PROFILES).await
    }

    /// The same once they say what the test is waiting for, or a panic saying
    /// what they did say instead.
    ///
    /// A poll rather than a wait on a signal, because what is being waited for
    /// is two hops away: A's store settles, A's stream says so, B announces it
    /// under A's id, and B's puller reads A's records over the link. What is
    /// asked throughout is the question the browser asks.
    async fn profiles_saying(&self, what: impl Fn(&[ProfileEntry]) -> bool) -> Vec<ProfileEntry> {
        let mut last = Vec::new();

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                last = self.profiles().await;

                if what(&last) {
                    return last.clone();
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("the Profiles never said it: {last:#?}"))
    }

    /// A Repo and a Conversation on it, for the pickers to be filled in on.
    async fn drafting(&self) -> i64 {
        let repo =
            store::register_repo(&self.pool, Path::new("/srv/verkstead"), "verkstead", "main")
                .await
                .unwrap()
                .expect("nothing is registered at that path yet")
                .id;

        store::start_conversation(&self.pool, repo, "amber-kestrel", self.device.id(), None)
            .await
            .unwrap()
            .expect("the Repo is registered")
    }

    /// All three of a Conversation's pickers filled in with one Pairing, the way
    /// the composer fills them.
    async fn picks(&self, conversation: i64, profile: &str) {
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
                &format!("/api/ui/conversations/{conversation}/{path}"),
                Some(saying),
            )
            .await;

            assert_eq!(said, "\"Chosen\"", "picking {path}");
        }
    }

    /// And what the press that starts the work answers.
    async fn starts(&self, conversation: i64) -> String {
        press(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}/grill"),
            None,
        )
        .await
    }
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

/// Every harness a device in this suite is normally taken to have, named the
/// way a session launches one.
const EVERY_HARNESS: &[&str] = &["claude", "codex", "grok", "opencode"];

/// The machine a device here is judged on: one directory on the `PATH` a
/// session would search, holding a program for each of `harnesses`.
///
/// **Stated rather than read off the runner.** A Profile not bound here whose
/// harness is not on this device reads broken in the onboarding probe's own
/// word, so what this box happens to have `claude` installed as is exactly the
/// thing this suite must not depend on: every device says what it has, and the
/// one test about a harness that is missing says it has none.
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

/// A and B, linked both ways, with B holding the streams — which is the device
/// every assertion here is made on.
///
/// Both ways because a link is both ways: B dials A pinned on A's fingerprint,
/// and A admits B because B's certificate is one it holds a membership for. And
/// linked before the streams are taken up, so the first take-up is the one that
/// pulls A's Profiles. Only B holds streams, so A hears nothing of B's — see
/// [`both_holding`] for the two ways.
async fn linked_up() -> (Verkstead, Verkstead, Holding) {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;

    let (machine, os) = this_machine();
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

    let holding = b.holding();

    (a, b, holding)
}

/// And the same with A holding streams to B as well, so a write pressed on B
/// reaches A.
async fn both_holding() -> (Verkstead, Verkstead, Holding, Holding) {
    let (a, b, holding) = linked_up().await;
    let other = a.holding();

    (a, b, holding, other)
}

/// What a device says about the machine it is on, read the way the server reads
/// it.
fn this_machine() -> (String, String) {
    (
        platform::hostname(),
        platform::os_word(Platform::Linux, None),
    )
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
    let (status, said) = pressing(app, path, saying).await;

    assert!(status.is_success(), "POST {path}: {status} {said}");

    said
}

/// The same press, with the status left for the caller to read.
///
/// What a press that was refused is read off.
async fn pressing(app: &Router, path: &str, saying: Option<&str>) -> (StatusCode, String) {
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

    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// What the press that starts the work answers over a Profile of A's that
/// cannot be run here, `broken` being which of the readings it is.
///
/// **The facts the row's own sentence is composed out of**, which is what
/// makes the refusal name what the human is already being shown: what is wrong
/// and what it runs. No device: a Profile has no home to name. Written out
/// rather than built with `json!`, that one sorting its keys and this being the
/// order the field is declared in.
fn refused_over(broken: &str) -> String {
    format!(
        "{{\"ProfileBroken\":{{\"broken\":\"{broken}\",\"agent_type\":\"Claude\",\
         \"device\":null}}}}"
    )
}

/// One row of a Profiles list, by the name on it.
fn called<'a>(rows: &'a [ProfileEntry], name: &str) -> Option<&'a ProfileEntry> {
    rows.iter().find(|row| row.name.as_deref() == Some(name))
}

/// Whether the row called `name` is here **with a binding of it**: the record
/// and the bindings are pulled apart, so a row can be drawn a moment before
/// any device's paths for it have arrived — empty paths, and no device to
/// fetch its account from.
fn landed(rows: &[ProfileEntry], name: &str) -> bool {
    called(rows, name).is_some_and(|row| match &row.account {
        verkstead_render::ProfileAccount::Claude { claude_dir, .. } => !claude_dir.is_empty(),
        verkstead_render::ProfileAccount::Codex { home }
        | verkstead_render::ProfileAccount::Grok { home }
        | verkstead_render::ProfileAccount::OpenCode { home } => !home.is_empty(),
    })
}

/// And what every row on it is called, for an assertion that says what it did
/// find.
fn names(rows: &[ProfileEntry]) -> Vec<&str> {
    rows.iter()
        .map(|row| row.name.as_deref().unwrap_or("Default"))
        .collect()
}

/// A Profile saved on A is on B's list within a Nudge, beside B's own and in
/// one list — with no binding on B and no device mark: its paths are A's, and
/// nothing about this filesystem is said of it.
#[tokio::test]
async fn a_members_profiles_are_listed_here_beside_this_devices_own() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;
    b.saves("here").await;

    let rows = b.profiles_saying(|rows| landed(rows, "work")).await;

    assert_eq!(names(&rows), vec!["here", "work"], "one list, by name");

    let theirs = called(&rows, "work").unwrap();

    assert!(!theirs.bound, "no binding of B's");
    assert_eq!(
        theirs.account,
        called(&a.profiles().await, "work").unwrap().account,
        "the paths on the row are A's binding's",
    );
    assert_eq!(
        theirs.broken, None,
        "and nothing about this filesystem is said of it: what a session here is \
         given is the account fetched from A before the launch — see \
         `tests/accounts.rs`",
    );
    assert_eq!(theirs.models, vec![MODEL.to_owned()], "what A said it runs");

    assert!(
        called(&rows, "here").unwrap().bound,
        "and B's own is bound here"
    );

    let json = serde_json::to_value(theirs).unwrap();
    assert!(
        json.get("device").is_none(),
        "no row carries a device mark: {json}",
    );
}

/// A Profile renamed on A reaches B on the next pull under the uuid it already
/// had: the uuid is what a Pairing here names, so it must not move.
#[tokio::test]
async fn a_rename_on_the_device_that_made_it_reaches_the_other_under_its_uuid() {
    let (a, b, _holding) = linked_up().await;

    let id = a.saves("work").await;

    b.profiles_saying(|rows| landed(rows, "work")).await;

    let conversation = b.drafting().await;
    b.picks(conversation, &id).await;

    a.renames(&id, "weekend").await;

    let rows = b
        .profiles_saying(|rows| called(rows, "weekend").is_some())
        .await;

    assert_eq!(names(&rows), vec!["weekend"], "renamed, and the one row");
    assert_eq!(called(&rows, "weekend").unwrap().id, id, "the same uuid");

    assert_eq!(
        b.paired(conversation).await.as_deref(),
        Some(id.as_str()),
        "and the Pairing made against it stands",
    );
}

/// Renamed on B, which holds no binding of it, the Profile redraws on A off the
/// record A pulls: the edit is a local write on B that replicates, and nothing
/// is relayed to a home.
#[tokio::test]
async fn a_rename_on_another_device_redraws_on_the_one_that_made_it() {
    let (a, b, _holding, _other) = both_holding().await;

    let id = a.saves("work").await;

    let landed = b.profiles_saying(|rows| landed(rows, "work")).await;
    let theirs = called(&landed, "work").unwrap().clone();

    let conversation = a.drafting().await;
    a.picks(conversation, &id).await;

    let (status, said) = b.saves_over(&theirs, Some("weekend")).await;

    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, "\"Saved\"");

    assert_eq!(
        names(&b.profiles().await),
        vec!["weekend"],
        "written on B at once",
    );

    let rows = a
        .profiles_saying(|rows| called(rows, "weekend").is_some())
        .await;
    let ours = called(&rows, "weekend").unwrap();

    assert_eq!(ours.id, id, "the same Profile");
    assert!(ours.bound, "and A's binding of it untouched");
    assert_eq!(
        ours.account, theirs.account,
        "its paths are A's, which a save on B leaves alone",
    );

    let drawn: ConversationView = reading(
        &a.workbench,
        &format!("/api/ui/conversations/{conversation}"),
    )
    .await;

    assert_eq!(
        drawn
            .implementation_pairing
            .expect("the Pairing on A survived the edit")
            .profile
            .name
            .as_deref(),
        Some("weekend"),
    );
}

/// A Profile removed on A is stamped removed on B too, and takes B's Pairings
/// that named it with it — exactly as a removal pressed on B would.
#[tokio::test]
async fn a_removal_on_one_device_stamps_it_removed_and_nulls_the_others_pairings() {
    let (a, b, _holding) = linked_up().await;

    let id = a.saves("work").await;
    a.saves("weekend").await;

    b.profiles_saying(|rows| rows.len() == 2).await;

    let conversation = b.drafting().await;
    b.picks(conversation, &id).await;

    a.removes(&id).await;

    let rows = b.profiles_saying(|rows| rows.len() == 1).await;

    assert_eq!(names(&rows), vec!["weekend"], "the one still live");

    b.paired_with(conversation, None).await;

    let (removed,): (Option<i64>,) =
        sqlx::query_as("SELECT deleted_at FROM replicated WHERE kind = 'profiles' AND uuid = ?")
            .bind(&id)
            .fetch_one(&b.pool)
            .await
            .unwrap();

    assert!(removed.is_some(), "kept on B as a deletion stamp");
}

/// And the work starts under a Profile bound on another device like it starts
/// under one bound here: what the launch does about it is fetch the account
/// from that device — see `tests/accounts.rs`, which is where a session really
/// runs under one.
#[tokio::test]
async fn the_work_starts_under_a_profile_bound_on_another_device() {
    let (a, b, _holding) = linked_up().await;

    let id = a.saves("work").await;

    b.profiles_saying(|rows| landed(rows, "work")).await;

    let conversation = b.drafting().await;
    b.picks(conversation, &id).await;

    assert_eq!(
        b.starts(conversation).await,
        "\"EmptyBrief\"",
        "the press is past the Pairings and stops at the Brief this fixture \
         never wrote",
    );
}

/// A member that is not answering keeps every record this device holds of it,
/// exactly as its Conversations are kept: a pull that could not be made changes
/// nothing. A Profile bound there alone reads unreachable.
#[tokio::test]
async fn a_member_that_stops_answering_keeps_its_records() {
    let b = Verkstead::answering(B).await;

    // A device that is a member and is not there: the identity is real, so the
    // membership row is a real one, and nothing is listening at the address.
    let away = tempfile::tempdir().unwrap();
    let asleep = Device::stated(away.path(), A).unwrap();

    b.linked_to(&asleep, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    // What it last served, which on a running server is what the last pull
    // merged.
    a_profile_of_as(&b, A).await;

    let _holding = b.holding();

    // Every membership there is, read again — which is what a device linked or
    // unlinked here sets going, and the one moment a member that is switched off
    // is dialled for its records.
    b.nudges.announce(Nudge::Devices);

    tokio::time::sleep(SETTLING).await;

    let rows = b.profiles().await;

    assert_eq!(names(&rows), vec!["work"], "the record stands");
    assert_eq!(
        rows[0].broken,
        Some(verkstead_render::Broken::NoBoundDevice),
        "and the device it is bound on is not answering",
    );

    let theirs = rows[0]
        .devices
        .iter()
        .find(|line| line.device.id.as_deref() == Some(A))
        .expect("the member keeps its line");

    assert!(!theirs.device.reachable, "drawn as not answering");
    assert_eq!(
        theirs.bound,
        Some(verkstead_render::ProfileAccount::Claude {
            claude_dir: "/home/you/accounts/work/.claude".to_owned(),
            config_file: "/home/you/accounts/work/.claude.json".to_owned(),
        }),
        "from the last binding of it this device holds",
    );
}

/// A device that is no longer a member takes nothing with it: a record is the
/// cluster's, and what this device merged from it stays.
#[tokio::test]
async fn the_profiles_of_an_unlinked_device_stay() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;
    b.saves("here").await;

    b.profiles_saying(|rows| landed(rows, "work")).await;

    forget_member(&b.pool, A).await.unwrap();
    b.nudges.announce(Nudge::Devices);

    tokio::time::sleep(SETTLING).await;

    assert_eq!(names(&b.profiles().await), vec!["here", "work"]);
}

/// The card carries a line per device of the cluster, saying how each runs the
/// Profile: A bound at the paths its binding holds, B from a copy — and B's
/// card says the same of the one record, off the binding it pulled.
#[tokio::test]
async fn the_card_says_where_each_device_is_bound_and_which_runs_from_a_copy() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;

    let theirs = b.profiles_saying(|rows| landed(rows, "work")).await;
    let ours = a.profiles().await;

    let on_a = called(&ours, "work").unwrap();
    let on_b = called(&theirs, "work").unwrap();
    let (machine, _) = this_machine();

    let lines = |row: &ProfileEntry| {
        row.devices
            .iter()
            .map(|line| {
                (
                    line.device.id.clone(),
                    line.device.name.clone(),
                    line.bound.clone(),
                )
            })
            .collect::<Vec<_>>()
    };

    assert_eq!(
        lines(on_a),
        vec![
            (None, A_MACHINE.to_owned(), Some(on_a.account.clone())),
            (Some(B.to_owned()), machine.clone(), None),
        ],
        "on A: this device first, bound at its paths, and B running from a copy",
    );
    assert_eq!(
        lines(on_b),
        vec![
            (None, machine, None),
            (
                Some(A.to_owned()),
                A_MACHINE.to_owned(),
                Some(on_a.account.clone())
            ),
        ],
        "and on B the same two facts, B's own line first",
    );
    assert!(
        on_b.devices.iter().all(|line| line.device.reachable),
        "both answering: {:#?}",
        on_b.devices,
    );
}

/// A Verkstead in no cluster draws its card as before, with no line at all; a
/// device linked to it gains its line on every card, and loses it once it is
/// unlinked — whatever binding it left behind.
#[tokio::test]
async fn a_device_linked_gains_its_line_and_one_unlinked_loses_it() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    b.saves("here").await;

    let alone = b.profiles().await;
    assert!(
        alone[0].devices.is_empty(),
        "no cluster, no device lines: {:#?}",
        alone[0].devices,
    );

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;

    let linked = b.profiles().await;
    let lines = &linked[0].devices;

    assert_eq!(lines.len(), 2, "this device and A: {lines:#?}");
    assert!(lines[0].device.id.is_none() && lines[0].bound.is_some());
    assert_eq!(lines[1].device.id.as_deref(), Some(A));
    assert_eq!(lines[1].device.name, A_MACHINE);
    assert_eq!(lines[1].device.os.as_deref(), Some(A_OS));
    assert_eq!(lines[1].bound, None, "A runs it from a copy");

    forget_member(&b.pool, A).await.unwrap();

    assert!(
        b.profiles().await[0].devices.is_empty(),
        "unlinked, and the line goes with the link",
    );
}

/// A Profile made on a third device reaches B through A, which holds it: every
/// device serves its whole table, whoever wrote a record.
#[tokio::test]
async fn a_third_devices_profile_reaches_here_through_a_member() {
    let (a, b, _holding) = linked_up().await;

    let c = Verkstead::answering(C).await;
    let (machine, os) = this_machine();

    a.linked_to(&c.device, &machine, &os, vec![c.at()]).await;
    c.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;

    let _a_holding = a.holding();

    c.saves("the-third").await;

    a.profiles_saying(|rows| called(rows, "the-third").is_some())
        .await;

    a.nudges.announce(Nudge::Profiles);

    let rows = b
        .profiles_saying(|rows| called(rows, "the-third").is_some())
        .await;

    assert!(
        !called(&rows, "the-third").unwrap().bound,
        "bound on C alone",
    );
}

/// A name another Profile of the harness has already is refused at the press on
/// whichever device it is pressed, against the whole table: B has no Profile
/// called `work` of its own, and refuses it all the same.
#[tokio::test]
async fn a_name_taken_on_another_device_is_refused_here() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;
    a.saves("weekend").await;

    let landed = b.profiles_saying(|rows| rows.len() == 2).await;
    let theirs = called(&landed, "weekend").unwrap().clone();

    let (status, said) = b.saves_over(&theirs, Some("work")).await;

    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, "\"NameTaken\"", "the first of the two rules");

    let (status, said) = b.saves_over(&theirs, None).await;

    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, "\"Saved\"", "and no unnamed Claude is held yet");

    a.saves_unnamed().await;

    let landed = b
        .profiles_saying(|rows| rows.iter().filter(|row| row.name.is_none()).count() == 2)
        .await;

    assert_eq!(
        landed.len(),
        3,
        "a pair arriving by replication is kept and drawn two alike",
    );

    let theirs = called(&landed, "work").unwrap().clone();
    let (_, said) = b.saves_over(&theirs, None).await;

    assert_eq!(
        said, "\"DefaultTaken\"",
        "and the second rule is the one an unnamed save hits",
    );
}

/// A removal pressed on B reaches every device of the cluster, each leaving the
/// Conversations that had picked it with nothing picked — C, linked to A
/// alone, by pulling A.
#[tokio::test]
async fn a_removal_pressed_here_reaches_every_device() {
    let (a, b, _holding, _other) = both_holding().await;

    let c = Verkstead::answering(C).await;
    let (machine, os) = this_machine();

    c.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;
    a.linked_to(&c.device, &machine, &os, vec![c.at()]).await;

    let _third = c.holding();

    let id = a.saves("work").await;

    b.profiles_saying(|rows| landed(rows, "work")).await;
    c.profiles_saying(|rows| landed(rows, "work")).await;

    let mut picked = Vec::new();

    for device in [&a, &b, &c] {
        let conversation = device.drafting().await;
        device.picks(conversation, &id).await;
        picked.push((device, conversation));
    }

    b.removes(&id).await;

    for (device, conversation) in picked {
        device
            .profiles_saying(|rows| called(rows, "work").is_none())
            .await;
        device.paired_with(conversation, None).await;
    }
}

/// With A not answering, an edit and a removal pressed on B are written on B
/// all the same: there is no home to put them to, and A takes them up when it
/// next pulls.
#[tokio::test]
async fn with_the_other_device_away_an_edit_and_a_removal_are_written_here() {
    let b = Verkstead::answering(B).await;

    let away = tempfile::tempdir().unwrap();
    let asleep = Device::stated(away.path(), A).unwrap();

    b.linked_to(&asleep, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    let id = a_profile_of_as(&b, A).await;

    let rows = b.profiles().await;
    let theirs = called(&rows, "work")
        .expect("the record A last served")
        .clone();

    let (status, said) = b.saves_over(&theirs, Some("weekend")).await;

    assert_eq!((status, said.as_str()), (StatusCode::OK, "\"Saved\""));
    assert_eq!(names(&b.profiles().await), vec!["weekend"]);

    let (status, said) = b.pressing_removal(&id).await;

    assert_eq!((status, said.as_str()), (StatusCode::OK, "\"Removed\""));
    assert!(b.profiles().await.is_empty());
}

/// A Profile of a harness this device has not got reads broken **in the
/// onboarding probe's own word** here, while the same Profile on the device it
/// is bound on is fine: whether a harness is on a machine is a fact about the
/// machine. And a Pairing made against it still reads as a Pairing, with a
/// broken Profile in it, and Start says why.
#[tokio::test]
async fn a_profile_whose_harness_is_absent_here_reads_broken_here_alone() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering_with_no_harness(B).await;

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;

    let (machine, os) = this_machine();
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

    let _holding = b.holding();

    let id = a.saves("work").await;

    assert_eq!(
        called(&a.profiles().await, "work")
            .expect("A's own row")
            .broken,
        None,
        "A has the harness, so there is nothing wrong with it there",
    );

    let landed = b.profiles_saying(|rows| landed(rows, "work")).await;

    assert_eq!(
        called(&landed, "work").unwrap().broken,
        Some(verkstead_render::Broken::HarnessMissing),
        "and B has no claude to run one under, whatever the account is",
    );

    let conversation = b.drafting().await;
    b.picks(conversation, &id).await;

    let opened: ConversationView = reading(
        &b.workbench,
        &format!("/api/ui/conversations/{conversation}"),
    )
    .await;

    let paired = opened
        .implementation_pairing
        .expect("the Pairing is the Conversation's own");

    assert_eq!(
        paired.profile.broken,
        Some(verkstead_render::Broken::HarnessMissing),
        "read as paired with a broken Profile rather than as unpaired",
    );
    assert!(!opened.ready_to_grill);

    assert_eq!(
        b.starts(conversation).await,
        refused_over("HarnessMissing"),
        "and Start says which harness rather than letting the launch find out",
    );
}

/// A Profile whose bound device has stopped answering reads unreachable, and
/// Start is refused over it: the account cannot be fetched, so a session
/// launched under it would come up logged out.
#[tokio::test]
async fn a_profile_whose_bound_device_is_not_answering_refuses_start() {
    let b = Verkstead::answering(B).await;

    let away = tempfile::tempdir().unwrap();
    let asleep = Device::stated(away.path(), A).unwrap();

    b.linked_to(&asleep, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    let id = a_profile_of_as(&b, A).await;

    let _holding = b.holding();

    // Every membership read again, which is the one moment a member that is
    // switched off is dialled — and what leaves the row drawn unreachable.
    b.nudges.announce(Nudge::Devices);
    tokio::time::sleep(SETTLING).await;

    let rows = b.profiles().await;

    assert_eq!(
        called(&rows, "work").unwrap().broken,
        Some(verkstead_render::Broken::NoBoundDevice),
    );

    let conversation = b.drafting().await;
    b.picks(conversation, &id).await;

    assert_eq!(b.starts(conversation).await, refused_over("NoBoundDevice"),);
}

/// A device bound on a Profile says on its binding whether the account there
/// holds a login file, and a binding without one is no source: B reads the
/// Profile as having no bound device to fetch from, naming none, and Start is
/// refused over it. A login made on A since is said on its binding at A's next
/// read, and B reads the Profile as one to run under.
#[tokio::test]
async fn a_binding_with_no_login_is_no_source_until_its_device_has_one() {
    let (a, b, _holding) = linked_up().await;

    let account = a.account_nobody_is_logged_in_to("work");
    let id = a.saving("work", account.clone()).await;

    let rows = b
        .profiles_saying(|rows| called(rows, "work").is_some_and(|row| row.broken.is_some()))
        .await;

    let row = called(&rows, "work").unwrap();
    assert_eq!(row.broken, Some(verkstead_render::Broken::NoBoundDevice));
    assert!(!row.login, "and nothing to copy anywhere");

    let conversation = b.drafting().await;
    b.picks(conversation, &id).await;

    assert_eq!(
        b.starts(conversation).await,
        refused_over("NoBoundDevice"),
        "refused naming the gap rather than coming up logged out",
    );

    // A login on A, which A's next read of its Profiles finds and says.
    std::fs::write(
        account.0.join(".credentials.json"),
        "{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-work\"}}",
    )
    .unwrap();

    a.profiles().await;

    let rows = b
        .profiles_saying(|rows| called(rows, "work").is_some_and(|row| row.broken.is_none()))
        .await;

    assert!(called(&rows, "work").unwrap().login);
}

/// A member a release apart whose Profile records this device cannot read has
/// that said on its Devices row, naming both releases and the one to update.
/// And the line goes once a pull of them lands.
#[tokio::test]
async fn a_members_profiles_that_cannot_be_read_are_said_on_its_row_with_the_gap() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    a.reading.now_running(Some("0.0.1"));
    a.garbles(true);

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;
    let (machine, os) = this_machine();
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

    let _holding = b.holding();

    b.trouble_on(
        A,
        Some(&format!(
            "{A_MACHINE}'s Agent Profiles couldn't be read. {A_MACHINE} is on 0.0.1 and this \
             device is on {RUNNING}. Update {A_MACHINE}."
        )),
    )
    .await;

    a.garbles(false);
    b.nudges.announce(Nudge::Devices);

    b.trouble_on(A, None).await;
}

/// On the same release the failure is said alone.
#[tokio::test]
async fn on_the_same_release_the_failure_is_said_alone() {
    let (a, b, _holding) = linked_up().await;

    a.garbles(true);
    b.nudges.announce(Nudge::Devices);

    b.trouble_on(
        A,
        Some(&format!("{A_MACHINE}'s Agent Profiles couldn't be read.")),
    )
    .await;
}

/// A member too old to serve the records route for Profiles is its row's
/// trouble line, naming both releases, and its Profiles are absent here — from
/// the Profiles section and so from every picker — until it updates.
#[tokio::test]
async fn a_member_too_old_to_serve_the_route_is_a_trouble_line_and_its_profiles_are_absent() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    a.reading.now_running(Some("0.0.1"));
    a.too_old(true);

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;
    let (machine, os) = this_machine();
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

    a.saves("work").await;

    let _holding = b.holding();

    b.trouble_on(
        A,
        Some(&format!(
            "{A_MACHINE}'s Agent Profiles couldn't be read. {A_MACHINE} is on 0.0.1 and this \
             device is on {RUNNING}. Update {A_MACHINE}."
        )),
    )
    .await;

    assert!(
        b.profiles().await.is_empty(),
        "A's Profile is in no list here",
    );

    // And it updates.
    a.too_old(false);
    a.reading.now_running(Some(RUNNING));
    b.nudges.announce(Nudge::Devices);

    b.profiles_saying(|rows| landed(rows, "work")).await;
    b.trouble_on(A, None).await;
}

/// And a member that says no release at all is from before releases were said,
/// so older than this device's.
#[tokio::test]
async fn a_member_that_says_no_release_is_on_an_older_one() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    a.reading.now_running(None);
    a.garbles(true);

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;
    let (machine, os) = this_machine();
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

    let _holding = b.holding();

    b.trouble_on(
        A,
        Some(&format!(
            "{A_MACHINE}'s Agent Profiles couldn't be read. {A_MACHINE} is on an older release \
             than this device's {RUNNING}. Update {A_MACHINE}."
        )),
    )
    .await;
}

/// Two Verksteads linked before this release, each with a Conversation paired
/// with the other's Profile, end with one record per Profile on both ends,
/// every Pairing intact and no mirror row left — with A updated first.
#[tokio::test]
async fn the_mirrors_of_before_this_release_are_bridged_when_a_updates_first() {
    bridged_whichever_updates_first(true).await;
}

/// And with B updated first.
#[tokio::test]
async fn the_mirrors_of_before_this_release_are_bridged_when_b_updates_first() {
    bridged_whichever_updates_first(false).await;
}

/// One of the pair is on this release and the other is not, which is a trouble
/// line on the first's row naming both releases, and the mirror it holds of the
/// second kept as it was, provisional, Pairing and all; then the second
/// updates, and both ends bridge their mirrors onto the records the other
/// publishes.
async fn bridged_whichever_updates_first(a_first: bool) {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    let (machine, os) = this_machine();

    let (first, second, second_id, second_name) = if a_first {
        (&a, &b, B, machine.as_str())
    } else {
        (&b, &a, A, A_MACHINE)
    };

    // A release apart, neither can read the other's records: the one behind
    // serves no records route, and the one ahead's records are nothing it asks
    // for. So both answer as too old from before they are linked, and the one
    // behind says the release it is on.
    a.too_old(true);
    b.too_old(true);
    second.reading.now_running(Some("0.0.1"));

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

    // Each device's own Profile, numbered before the uuids, and the other's
    // mirror of it under a provisional uuid with the integer beside it.
    let on_a = a.saves_numbered("on-a", 7).await;
    let on_b = b.saves_numbered("on-b", 9).await;

    let on_a_here = a.drafting().await;
    let on_b_here = b.drafting().await;

    let held_of_b = a.mirrors_from_before(on_a_here, B, 9).await;
    let held_of_a = b.mirrors_from_before(on_b_here, A, 7).await;

    // And a member's account fetched into the mirror under its provisional
    // uuid, before the switch, on B.
    let fetched = b.account_mirror(&held_of_a);
    std::fs::create_dir_all(&fetched).unwrap();
    std::fs::write(fetched.join(".claude.json"), "{}\n").unwrap();

    let _first_holding = first.holding();

    first
        .trouble_on(
            second_id,
            Some(&format!(
                "{second_name}'s Agent Profiles couldn't be read. {second_name} is on 0.0.1 and \
                 this device is on {RUNNING}. Update {second_name}."
            )),
        )
        .await;

    let (first_conversation, first_held) = if a_first {
        (on_a_here, &held_of_b)
    } else {
        (on_b_here, &held_of_a)
    };

    assert_eq!(
        first.names_in_implementation(first_conversation).await,
        Some(first_held.clone()),
        "the mirror of the member a release behind is held as it was, Pairing and all",
    );
    assert_eq!(
        first.profiles().await.len(),
        1,
        "and is in no list, its Profiles absent until it updates",
    );

    // And the second updates, after which each pulls the other.
    a.too_old(false);
    b.too_old(false);
    second.reading.now_running(Some(RUNNING));

    let _second_holding = second.holding();
    first.nudges.announce(Nudge::Devices);

    a.paired_with(on_a_here, Some(&on_b)).await;
    b.paired_with(on_b_here, Some(&on_a)).await;

    for (device, own, theirs) in [(&a, &on_a, &on_b), (&b, &on_b, &on_a)] {
        let rows = device
            .profiles_saying(|rows| rows.iter().any(|row| &row.id == theirs))
            .await;

        let mut ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        ids.sort();

        let mut both = vec![own.as_str(), theirs.as_str()];
        both.sort();

        assert_eq!(
            ids, both,
            "one record per Profile, by the uuid it was minted"
        );

        let (left,): (i64,) = sqlx::query_as("SELECT count(*) FROM profiles")
            .fetch_one(&device.pool)
            .await
            .unwrap();

        assert_eq!(left, 0, "and no mirror row left");
    }

    assert!(
        !fetched.exists(),
        "the account copy is no longer under the provisional uuid"
    );
    assert!(
        b.account_mirror(&on_a).join(".claude.json").exists(),
        "it followed the rewrite, so the next session fetches into it",
    );
}

/// A Profile called `work` of `home`'s, written down on `on` as the record and
/// the binding `home` last served. The uuid is given back.
async fn a_profile_of_as(on: &Verkstead, home: &str) -> String {
    let id = "7e0f1d2c-0000-4000-8000-000000000007";

    store::profile_arrived(
        &on.pool,
        id,
        home,
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

    id.to_owned()
}

impl Verkstead {
    /// A Profile saved on this device and numbered `was` before the uuids,
    /// which is what the switch leaves of one: where it was minted and the id it
    /// had there.
    async fn saves_numbered(&self, name: &str, was: i64) -> String {
        let id = self.saves(name).await;

        sqlx::query(
            "UPDATE replicated SET body = json_set(body, '$.minted_on', ?, '$.minted_id', ?)
             WHERE kind = 'profiles' AND uuid = ?",
        )
        .bind(self.device.id())
        .bind(was)
        .bind(&id)
        .execute(&self.pool)
        .await
        .unwrap();

        id
    }

    /// And a mirror of `home`'s Profile `was` as a database from before this
    /// release holds one: a row of the old table under a provisional uuid of
    /// this device's minting, with the home and the integer beside it — and
    /// `conversation`'s implementation picker naming it, which is what is to
    /// survive. The provisional uuid is given back.
    async fn mirrors_from_before(&self, conversation: i64, home: &str, was: i64) -> String {
        let provisional = format!("0000000{was}-0000-4000-8000-{}", &self.device.id()[..12]);

        sqlx::query(
            "INSERT INTO profiles
                 (id, name, claude_dir, config_file, model, agent_type, memory,
                  home_device, home_id)
             VALUES (?, 'theirs', '/elsewhere/.claude', '/elsewhere/.claude.json', ?, 'claude',
                     1, ?, ?)",
        )
        .bind(&provisional)
        .bind(MODEL)
        .bind(home)
        .bind(was)
        .execute(&self.pool)
        .await
        .unwrap();

        sqlx::query("UPDATE conversations SET implementation_profile_id = ? WHERE id = ?")
            .bind(&provisional)
            .bind(conversation)
            .execute(&self.pool)
            .await
            .unwrap();

        provisional
    }

    /// Where this device keeps the account copy of Profile `id`.
    fn account_mirror(&self, id: &str) -> PathBuf {
        self.dir.path().join("data").join("accounts").join(id)
    }

    /// The uuid a Conversation's implementation picker names, whether or not a
    /// Profile is there under it — read off the column.
    async fn names_in_implementation(&self, conversation: i64) -> Option<String> {
        let (named,): (Option<String>,) =
            sqlx::query_as("SELECT implementation_profile_id FROM conversations WHERE id = ?")
                .bind(conversation)
                .fetch_one(&self.pool)
                .await
                .unwrap();

        named
    }

    /// The Profile a Conversation's implementation picker draws.
    async fn paired(&self, conversation: i64) -> Option<String> {
        let drawn: ConversationView = reading(
            &self.workbench,
            &format!("/api/ui/conversations/{conversation}"),
        )
        .await;

        drawn
            .implementation_pairing
            .map(|pairing| pairing.profile.id)
    }

    /// Wait until it draws `profile` — or nothing picked, for `None` — or a
    /// panic saying what it drew.
    async fn paired_with(&self, conversation: i64, profile: Option<&str>) {
        let mut last = None;

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                last = self.paired(conversation).await;

                if last.as_deref() == profile {
                    return;
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("the Pairing never drew {profile:?}: it drew {last:?}"));
    }
}
