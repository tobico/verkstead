//! **Mirror rows**: every device's Profiles section and every pairing picker
//! listing every member's accounts beside its own (ADR-0020, *Shared
//! Profiles*).
//!
//! **Two Verksteads, a real dial and a real stream.** `tests/relaying.rs` is the
//! hop a browser makes through a device and `tests/freshness.rs` is the news
//! coming back over it; this is the account list written down out of both. So
//! nothing here hands B a list of A's by the back door: B reads A's own
//! `/api/ui/profiles` over the Peer Listener, refreshed off the Nudge stream it
//! holds to A, and what every assertion reads is B's own `/api/ui/profiles` —
//! the one an open Profiles section and every picker ask for.
//!
//! **And what the mirrors are is rows in B's store**, which is the whole of why
//! they exist: a Pairing, a Repo's memory and a Conversation all name a Profile
//! by a **local** id, and a mirror is what gives one of A's accounts an id here.
//! So the assertions about a removal are assertions about B's Conversations, and
//! the one about a refresh is that the id did not move.

use std::net::SocketAddr;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{ConversationView, ProfileEntry};
use verkstead_schema::Nudge;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Environment, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::{Routers, open_database, routers_answering_devices_telling, store};
use verkstead_store::{Linking, Mirror, forget_member, record_member};

/// The device whose accounts are mirrored.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the device that mirrors them, which is where every assertion is made.
const B: &str = "0011223344556677889900aabbccddee";

/// And a third, which nobody in these tests ever links to: what it is for is
/// the row A holds of *it*, which must not travel on to B.
const C: &str = "ff11223344556677889900aabbccddee";

/// What A is written down as on B: the name and the OS word B holds against A's
/// row, which is what a mirrored row says.
///
/// Stated rather than read off a machine, because they are the membership row's
/// — a member's name and OS are what it last said about itself at an exchange,
/// and nothing in a cluster asks a device for them again. And deliberately not
/// this box's own words: every device in this suite is one machine, so a row
/// that took B's hostname for A's would read right for the wrong reason.
const A_MACHINE: &str = "the-workstation";
const A_OS: &str = "macOS 15.1";

/// The Profiles section and every pairing picker, which is the path every
/// assertion here reads.
const PROFILES: &str = "/api/ui/profiles";

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
    /// thing about a **mirror** that is a fact about the device drawing it
    /// rather than about the account.
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
        } = routers_answering_devices_telling(
            pool.clone(),
            cluster.clone(),
            nudges.clone(),
            machine,
        );

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
        }
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

    /// An account on this machine for a Profile to name: the pair Claude Code
    /// keeps one as, really made, because saving a Profile is refused where the
    /// paths are not there.
    ///
    /// **Logged in**, which is what an account somebody uses is: the login is
    /// the one file of it another device can be lent, so an account with none
    /// is a Profile that cannot be used away from here at all — see
    /// [`Verkstead::account_nobody_is_logged_in_to`], which is that case on
    /// purpose.
    fn account(&self, name: &str) -> (PathBuf, PathBuf) {
        let (claude_dir, config_file) = self.account_nobody_is_logged_in_to(name);

        std::fs::write(
            claude_dir.join(".credentials.json"),
            "{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-work\"}}",
        )
        .unwrap();

        (claude_dir, config_file)
    }

    /// And the same account with no login file in it, which is what a Claude
    /// login kept in the macOS Keychain leaves behind — and what a sign-out at
    /// home comes to.
    fn account_nobody_is_logged_in_to(&self, name: &str) -> (PathBuf, PathBuf) {
        let under = self.dir.path().join("accounts").join(name);
        let claude_dir = under.join(".claude");
        let config_file = under.join(".claude.json");

        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(&config_file, "{}\n").unwrap();

        (claude_dir, config_file)
    }

    /// A Profile saved on this device, the way the form saves one.
    async fn saves(&self, name: &str) -> i64 {
        let account = self.account(name);

        self.saving(name, account).await
    }

    /// And one over an account nobody is logged in to, which is a Profile that
    /// is perfectly runnable here and cannot be used away at all.
    async fn saves_logged_out(&self, name: &str) -> i64 {
        let account = self.account_nobody_is_logged_in_to(name);

        self.saving(name, account).await
    }

    /// Both of them: the form's own press, over whichever account was made.
    async fn saving(&self, name: &str, (claude_dir, config_file): (PathBuf, PathBuf)) -> i64 {
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
    async fn renames(&self, id: i64, to: &str) {
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
    async fn removes(&self, id: i64) {
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
    /// whether the row is this device's own or a **mirror** of a member's. The
    /// paths of a mirror are the home machine's, so the form sending them back is
    /// what sends that machine its own account.
    ///
    /// The status is given back with the answer, because a press over a mirror can
    /// be refused by the hop rather than answered by the endpoint.
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
    async fn pressing_removal(&self, id: i64) -> (StatusCode, String) {
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
    /// under A's id, and B's refresher reads A's Profiles over the link. What is
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

        store::start_conversation(&self.pool, repo, "amber-kestrel", self.device.id())
            .await
            .unwrap()
            .expect("the Repo is registered")
    }

    /// All three of a Conversation's pickers filled in with one Pairing, the way
    /// the composer fills them.
    async fn picks(&self, conversation: i64, profile: i64) {
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
/// **Stated rather than read off the runner.** A **mirror** of a Profile whose
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
/// reads A's Profiles.
async fn linked_up() -> (Verkstead, Verkstead, Holding) {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;

    let (machine, os) = this_machine();
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

    let holding = b.holding();

    (a, b, holding)
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
/// What a press that was *refused* is read off: a press over a **mirror** is put
/// to the device the Profile is at home on, and a machine that did not take it is
/// a status and a sentence rather than one of the endpoint's own words — see
/// `relaying::Refusal`.
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
/// **The three facts the row's own sentence is composed out of**, which is what
/// makes the refusal name what the human is already being shown: what is wrong,
/// what it runs, and the machine the account is at home on. Written out rather
/// than built with `json!`, that one sorting its keys and this being the order
/// the field is declared in.
fn refused_over(broken: &str) -> String {
    format!(
        "{{\"ProfileBroken\":{{\"broken\":\"{broken}\",\"agent_type\":\"Claude\",\
         \"device\":\"{A_MACHINE}\"}}}}"
    )
}

/// One row of a Profiles list, by the name on it.
fn called<'a>(rows: &'a [ProfileEntry], name: &str) -> Option<&'a ProfileEntry> {
    rows.iter().find(|row| row.name.as_deref() == Some(name))
}

/// And what every row on it is called, for an assertion that says what it did
/// find.
fn names(rows: &[ProfileEntry]) -> Vec<&str> {
    rows.iter()
        .map(|row| row.name.as_deref().unwrap_or("Default"))
        .collect()
}

/// A Profile saved on A is on B's list within a Nudge, beside B's own — and the
/// row says it is A's, by A's own name and OS word, and that its account is not
/// on this device.
#[tokio::test]
async fn a_members_profiles_are_listed_here_beside_this_devices_own() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;
    b.saves("here").await;

    let rows = b
        .profiles_saying(|rows| called(rows, "work").is_some())
        .await;

    assert_eq!(names(&rows), vec!["here", "work"], "one list, by name");

    let mirror = called(&rows, "work").unwrap();
    let device = mirror
        .device
        .as_ref()
        .expect("a mirror says which device it is at home on");

    assert_eq!(device.id.as_deref(), Some(A));
    assert_eq!(device.name, A_MACHINE, "A's own name, off B's membership");
    assert_eq!(device.os, A_OS, "and the word its mark is drawn from");
    assert!(device.reachable, "and A is answering");

    assert_eq!(
        mirror.broken, None,
        "and nothing about this filesystem is said of it: the account is on A, \
         and what a session here is given is a mirror of it fetched before the \
         launch — see `tests/accounts.rs`",
    );

    assert_eq!(mirror.models, vec![MODEL.to_owned()], "what A said it runs");

    assert_eq!(
        called(&rows, "here").unwrap().device,
        None,
        "and this device's own rows are its own",
    );
}

/// A Profile renamed on A reaches B on the refresh, on the row it already had:
/// the local id is what a Pairing here names, so it must not move.
#[tokio::test]
async fn a_rename_at_home_reaches_the_other_device_without_moving_the_row() {
    let (a, b, _holding) = linked_up().await;

    let there = a.saves("work").await;

    let landed = b
        .profiles_saying(|rows| called(rows, "work").is_some())
        .await;
    let here = called(&landed, "work").unwrap().id;

    let conversation = b.drafting().await;
    b.picks(conversation, here).await;

    a.renames(there, "weekend").await;

    let rows = b
        .profiles_saying(|rows| called(rows, "weekend").is_some())
        .await;

    assert_eq!(names(&rows), vec!["weekend"], "renamed, and the one row");
    assert_eq!(
        called(&rows, "weekend").unwrap().id,
        here,
        "on the row it already had",
    );

    let drawn: ConversationView = reading(
        &b.workbench,
        &format!("/api/ui/conversations/{conversation}"),
    )
    .await;

    assert_eq!(
        drawn
            .implementation_pairing
            .expect("the Pairing made against the mirror survived the refresh")
            .profile
            .id,
        here,
    );
}

/// A Profile removed on A leaves B on the next refresh, and takes the Pairings
/// that named it with it — exactly as a removal pressed on B would.
#[tokio::test]
async fn a_removal_at_home_takes_the_mirror_and_nulls_the_pairings_that_named_it() {
    let (a, b, _holding) = linked_up().await;

    let there = a.saves("work").await;
    a.saves("weekend").await;

    let landed = b.profiles_saying(|rows| rows.len() == 2).await;
    let here = called(&landed, "work").unwrap().id;

    let conversation = b.drafting().await;
    b.picks(conversation, here).await;

    a.removes(there).await;

    let rows = b.profiles_saying(|rows| rows.len() == 1).await;

    assert_eq!(names(&rows), vec!["weekend"], "the one A still holds");

    let drawn: ConversationView = reading(
        &b.workbench,
        &format!("/api/ui/conversations/{conversation}"),
    )
    .await;

    assert!(
        drawn.implementation_pairing.is_none(),
        "the Conversation that had picked it is left with an empty picker",
    );
}

/// And the work starts under a mirror like it starts under any other row: the
/// account is on A, and what the launch does about that is fetch a mirror of it
/// under this device's Data Directory — see `tests/accounts.rs`, which is where a
/// session really runs under one.
#[tokio::test]
async fn the_work_starts_under_a_mirror() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;

    let landed = b
        .profiles_saying(|rows| called(rows, "work").is_some())
        .await;
    let here = called(&landed, "work").unwrap().id;

    let conversation = b.drafting().await;
    b.picks(conversation, here).await;

    assert_eq!(
        b.starts(conversation).await,
        "\"EmptyBrief\"",
        "the press is past the Pairings — a member's account is one to run the \
         work under — and stops at the Brief this fixture never wrote",
    );
}

/// A member that is not answering keeps the rows it last gave, exactly as its
/// Conversations are kept: a read that could not be made changes nothing.
#[tokio::test]
async fn a_member_that_stops_answering_keeps_its_rows() {
    let b = Verkstead::answering(B).await;

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
        &Mirror {
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

    let _holding = b.holding();

    // Every membership there is, read again — which is what a device linked or
    // unlinked here sets going, and the one moment a member that is switched off
    // is dialled for its Profiles.
    b.nudges.announce(Nudge::Devices);

    tokio::time::sleep(SETTLING).await;

    let rows = b.profiles().await;

    assert_eq!(names(&rows), vec!["work"], "the row A last gave stands");
    assert!(
        !rows[0].device.as_ref().expect("it is still A's").reachable,
        "drawn as the machine that did not answer",
    );
}

/// The mirrors of a device that is no longer a member go: what prunes them is
/// the membership, exactly as it prunes the rows the merged list holds.
#[tokio::test]
async fn the_mirrors_of_an_unlinked_device_go() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;
    b.saves("here").await;

    b.profiles_saying(|rows| called(rows, "work").is_some())
        .await;

    forget_member(&b.pool, A).await.unwrap();
    b.nudges.announce(Nudge::Devices);

    let rows = b
        .profiles_saying(|rows| called(rows, "work").is_none())
        .await;

    assert_eq!(names(&rows), vec!["here"], "this device's own is untouched");
}

/// What a member is mirroring is not taken in: a cluster of three has A holding
/// rows of C's, and B takes A's own accounts and nothing else.
///
/// Otherwise every device in a cluster of three would hold two rows for one
/// account — and B would take its own accounts back again under A's name.
#[tokio::test]
async fn a_members_own_profiles_are_taken_and_its_mirrors_are_not() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;

    // A row of A's that is at home somewhere else, written the way A's own
    // refresher writes one.
    store::record_mirror(
        &a.pool,
        &Mirror {
            device: C.to_owned(),
            id: 3,
            login: true,
        },
        &store::ProfileFacts {
            name: Some("the-third".to_owned()),
            account: store::Account::Claude {
                claude_dir: PathBuf::from("/home/you/accounts/third/.claude"),
                config_file: PathBuf::from("/home/you/accounts/third/.claude.json"),
            },
            models: vec![MODEL.to_owned()],
            memory: true,
        },
    )
    .await
    .unwrap();

    a.nudges.announce(Nudge::Profiles);

    b.profiles_saying(|rows| called(rows, "work").is_some())
        .await;

    // And long enough for a row that was going to arrive to have arrived: the
    // refresh that brought `work` over is the same read that saw the other one.
    tokio::time::sleep(SETTLING).await;

    assert_eq!(
        names(&b.profiles().await),
        vec!["work"],
        "A's own account, and nothing A is itself mirroring",
    );
}

/// An edit made on B lands in A's own Profiles, and both devices' rows redraw off
/// the row A saved rather than off what was typed here.
///
/// The form over a mirror saves — which is the human's choice over editing an
/// account only where it lives — and the save is one hop: put to A as the ordinary
/// edit of A's own Profile, addressed by the id the mirror records for it there.
#[tokio::test]
async fn an_edit_made_here_lands_in_the_home_devices_own_profiles() {
    let (a, b, _holding) = linked_up().await;

    let there = a.saves("work").await;

    let landed = b
        .profiles_saying(|rows| called(rows, "work").is_some())
        .await;
    let mirror = called(&landed, "work").unwrap().clone();

    // A Pairing made against the mirror, so that *the row did not move* is an
    // assertion about something that names it.
    let conversation = b.drafting().await;
    b.picks(conversation, mirror.id).await;

    let (status, said) = b.saves_over(&mirror, Some("weekend")).await;

    assert_eq!(status, StatusCode::OK, "A took it: {said}");
    assert_eq!(said, "\"Saved\"", "and the answer is A's own word for it");

    // A's own list, which is where the account is.
    let at_home = a.profiles().await;

    assert_eq!(names(&at_home), vec!["weekend"], "renamed at home");
    assert_eq!(
        called(&at_home, "weekend").unwrap().id,
        there,
        "on A's own row rather than as a second Profile",
    );
    assert_eq!(
        called(&at_home, "weekend").unwrap().device,
        None,
        "and it is still A's own rather than something A is mirroring",
    );

    // And B's, which is the mirror redrawn off the saved row.
    let here = b
        .profiles_saying(|rows| called(rows, "weekend").is_some())
        .await;

    assert_eq!(names(&here), vec!["weekend"], "the one row, renamed");
    assert_eq!(
        called(&here, "weekend").unwrap().id,
        mirror.id,
        "on the local row it already had, so the Pairing against it stands",
    );

    let drawn: ConversationView = reading(
        &b.workbench,
        &format!("/api/ui/conversations/{conversation}"),
    )
    .await;

    assert_eq!(
        drawn
            .implementation_pairing
            .expect("the Pairing made against the mirror survived the edit")
            .profile
            .name
            .as_deref(),
        Some("weekend"),
    );
}

/// A name already taken at home is a name already taken: the refusal is the word
/// A's own store turned the write away with, which is the word a clash pressed on
/// A itself would give.
///
/// The first of the two uniqueness rules, and it is A's to hold — B has no Profile
/// called `work` at all, so nothing on this device could have refused this.
#[tokio::test]
async fn a_name_taken_at_home_comes_back_as_the_refusal_a_local_clash_gives() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;
    a.saves("weekend").await;

    let landed = b
        .profiles_saying(|rows| called(rows, "weekend").is_some())
        .await;
    let mirror = called(&landed, "weekend").unwrap().clone();

    let (status, said) = b.saves_over(&mirror, Some("work")).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "a clash is an outcome of the press rather than a failure of the hop",
    );
    assert_eq!(
        said, "\"NameTaken\"",
        "the first of the two rules, named as a local clash names it",
    );

    assert_eq!(
        names(&a.profiles().await),
        vec!["weekend", "work"],
        "and nothing at home moved",
    );
}

/// And the other of the two rules, which is the one a Profile with no name can
/// hit: A already holds the one unnamed Claude account.
///
/// Which of the two refused a save is read off what was being saved, so a mirror
/// saved with the name taken out has to come back as the second of them rather
/// than as the first.
#[tokio::test]
async fn an_unnamed_save_that_clashes_at_home_names_the_other_rule() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;
    a.saves_unnamed().await;

    let landed = b
        .profiles_saying(|rows| called(rows, "work").is_some())
        .await;
    let mirror = called(&landed, "work").unwrap().clone();

    let (status, said) = b.saves_over(&mirror, None).await;

    assert_eq!(status, StatusCode::OK, "A took it and refused it: {said}");
    assert_eq!(
        said, "\"DefaultTaken\"",
        "the second of the two rules, in A's own word",
    );
}

/// A removal pressed on B takes the Profile off A — and the mirror off every
/// device of the cluster, each leaving the Conversations that had picked it with
/// nothing picked.
///
/// **A third device is what makes the claim worth making.** C mirrors A as B does
/// and nothing here ever tells C anything: it learns of the removal by refreshing
/// its own mirror off A's news, which is the rule every announcement in a cluster
/// is held under. Nothing is relayed twice.
#[tokio::test]
async fn a_removal_pressed_here_takes_the_profile_off_home_and_every_mirror_with_it() {
    let (a, b, _holding) = linked_up().await;

    // The third device, linked to A both ways and holding its own streams, exactly
    // as B is.
    let c = Verkstead::answering(C).await;
    let (machine, os) = this_machine();

    c.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;
    a.linked_to(&c.device, &machine, &os, vec![c.at()]).await;

    let _third = c.holding();

    let there = a.saves("work").await;

    let on_b = b
        .profiles_saying(|rows| called(rows, "work").is_some())
        .await;
    let on_c = c
        .profiles_saying(|rows| called(rows, "work").is_some())
        .await;

    let here = called(&on_b, "work").unwrap().id;
    let over_there = called(&on_c, "work").unwrap().id;

    // One Conversation per device, each having picked the account: on A its own
    // row, and on the other two the mirror of it.
    let at_a = a.drafting().await;
    a.picks(at_a, there).await;

    let at_b = b.drafting().await;
    b.picks(at_b, here).await;

    let at_c = c.drafting().await;
    c.picks(at_c, over_there).await;

    // The press, made over the mirror on the device whose account it is not.
    b.removes(here).await;

    assert!(
        called(&a.profiles().await, "work").is_none(),
        "off the device its account lives on",
    );

    b.profiles_saying(|rows| called(rows, "work").is_none())
        .await;
    c.profiles_saying(|rows| called(rows, "work").is_none())
        .await;

    for (device, conversation, whose) in [(&a, at_a, "A"), (&b, at_b, "B"), (&c, at_c, "C")] {
        let drawn: ConversationView = reading(
            &device.workbench,
            &format!("/api/ui/conversations/{conversation}"),
        )
        .await;

        assert!(
            drawn.implementation_pairing.is_none(),
            "the Conversation on {whose} reads as one nothing has been picked for",
        );
    }
}

/// With A not answering, both the save and the removal are refused naming the
/// machine, and nothing on this device has moved.
///
/// A mirror edited against a machine that is not there would be a row disagreeing
/// with the account it stands for, so the press is refused rather than written down
/// here and put over later.
#[tokio::test]
async fn with_the_home_device_away_both_presses_are_refused_naming_it() {
    let b = Verkstead::answering(B).await;

    // A device that is a member and is not there: the identity is real, so the
    // membership row is a real one, and nothing is listening at the address.
    let away = tempfile::tempdir().unwrap();
    let asleep = Device::stated(away.path(), A).unwrap();

    b.linked_to(&asleep, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    // What A last gave, which on a running server is what the last refresh wrote
    // down.
    store::record_mirror(
        &b.pool,
        &Mirror {
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

    let rows = b.profiles().await;
    let mirror = called(&rows, "work").expect("the row A last gave").clone();

    let (status, said) = b.saves_over(&mirror, Some("weekend")).await;

    assert_eq!(
        status,
        StatusCode::BAD_GATEWAY,
        "the hop refused it rather than the endpoint answering: {said}",
    );
    assert!(
        said.contains(A_MACHINE),
        "the machine is named by the name the human gave it: {said}",
    );

    let (status, said) = b.pressing_removal(mirror.id).await;

    assert_eq!(status, StatusCode::BAD_GATEWAY, "and so is the removal");
    assert!(said.contains(A_MACHINE), "named the same way: {said}");

    let rows = b.profiles().await;

    assert_eq!(
        names(&rows),
        vec!["work"],
        "and the row stands as A last gave it",
    );
    assert_eq!(
        called(&rows, "work").unwrap().id,
        mirror.id,
        "on the id it had, so nothing a Pairing names has moved",
    );
}

/// A Profile whose account at home holds no login file cannot be used away from
/// that machine at all, and every device's row says so — including the one the
/// account is on, where the Profile is still perfectly runnable.
///
/// A Claude login kept in the macOS Keychain is the case this exists for, and a
/// sign-out at home is the same thing arrived at from the other direction. The
/// fix is a login on the home device, which is why the row draws the machine
/// beside the trouble.
#[tokio::test]
async fn a_profile_with_no_login_at_home_reads_as_not_usable_away() {
    let (a, b, _holding) = linked_up().await;

    a.saves_logged_out("keychain").await;

    let here = a.profiles().await;
    let at_home = called(&here, "keychain").expect("A's own row");

    assert_eq!(
        at_home.broken, None,
        "nothing is wrong with it where the account is: a login in the Keychain \
         is a login, and a session at home is launched under it as ever",
    );
    assert!(
        !at_home.login,
        "and the row says it has no login file to lend, which is what it cannot \
         be used away without",
    );

    let landed = b
        .profiles_saying(|rows| called(rows, "keychain").is_some())
        .await;
    let mirror = called(&landed, "keychain").expect("the mirror of it on B");

    assert_eq!(
        mirror.broken,
        Some(verkstead_render::Broken::NoLoginAtHome),
        "and away from home that is the whole of why it cannot be run: there is \
         nothing for B to mirror",
    );
    assert!(!mirror.login, "said the same way on both devices");

    // Refused rather than hidden: the row is still on the list and still in the
    // picker, because a row saying why it cannot be run is something to go and
    // put right.
    assert_eq!(names(&landed), vec!["keychain"]);

    let conversation = b.drafting().await;
    b.picks(conversation, mirror.id).await;

    assert_eq!(
        b.starts(conversation).await,
        refused_over("NoLoginAtHome"),
        "and Start is refused naming it, before a session comes up logged out \
         with nothing on the Timeline saying why",
    );
}

/// A login made at home after the fact is a mirror that stops reading broken:
/// the fix for *no login at home* is a login on the home device, and nothing
/// here has to be pressed for it to take.
#[tokio::test]
async fn a_login_at_home_puts_the_mirror_right() {
    let (a, b, _holding) = linked_up().await;

    a.saves_logged_out("keychain").await;

    b.profiles_saying(|rows| {
        called(rows, "keychain")
            .is_some_and(|row| row.broken == Some(verkstead_render::Broken::NoLoginAtHome))
    })
    .await;

    // The login arriving, and A saying its Profiles moved — which is a `claude
    // /login` at home followed by whatever next makes A announce. The refresh
    // is what carries the fact, and nothing here is pressed for it.
    a.account("keychain");
    a.nudges.announce(Nudge::Profiles);

    let landed = b
        .profiles_saying(|rows| called(rows, "keychain").is_some_and(|row| row.login))
        .await;

    assert_eq!(
        called(&landed, "keychain").unwrap().broken,
        None,
        "with a login to lend there is nothing in the way of a session here",
    );
}

/// A mirror of a harness this device has not got reads broken **in the
/// onboarding probe's own word**, while the same Profile on its home device is
/// fine: whether a harness is on a machine is a fact about the machine, and the
/// account is not on this one.
#[tokio::test]
async fn a_mirror_whose_harness_is_absent_here_reads_broken_here_alone() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering_with_no_harness(B).await;

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;

    let (machine, os) = this_machine();
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

    let _holding = b.holding();

    a.saves("work").await;

    let here = a.profiles().await;

    assert_eq!(
        called(&here, "work").expect("A's own row").broken,
        None,
        "A has the harness, so there is nothing wrong with it there",
    );

    let landed = b
        .profiles_saying(|rows| called(rows, "work").is_some())
        .await;
    let mirror = called(&landed, "work").expect("the mirror of it on B");

    assert_eq!(
        mirror.broken,
        Some(verkstead_render::Broken::HarnessMissing),
        "and B has no claude to run one under, whatever the account is",
    );
    assert_eq!(
        names(&landed),
        vec!["work"],
        "and the row stays on the list"
    );

    let conversation = b.drafting().await;
    b.picks(conversation, mirror.id).await;

    assert_eq!(
        b.starts(conversation).await,
        refused_over("HarnessMissing"),
        "and Start says which harness rather than letting the launch find out",
    );
}

/// A mirror whose home has stopped answering reads unreachable — the way that
/// device reads in the sidebar — and Start names the machine: the account
/// cannot be fetched, so a session launched under it would come up logged out.
#[tokio::test]
async fn a_mirror_whose_home_is_not_answering_reads_unreachable_and_refuses_start() {
    let b = Verkstead::answering(B).await;

    // A device that is a member and is not there: the identity is real, so the
    // membership row is a real one, and nothing is listening at the address.
    let away = tempfile::tempdir().unwrap();
    let asleep = Device::stated(away.path(), A).unwrap();

    b.linked_to(&asleep, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    // What A last gave, which on a running server is what the last refresh
    // wrote down — a login and all, because what is wrong is not the account.
    store::record_mirror(
        &b.pool,
        &Mirror {
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

    let _holding = b.holding();

    // Every membership read again, which is the one moment a member that is
    // switched off is dialled — and what leaves the row drawn unreachable.
    b.nudges.announce(Nudge::Devices);
    tokio::time::sleep(SETTLING).await;

    let rows = b.profiles().await;
    let mirror = called(&rows, "work").expect("the row A last gave");

    assert_eq!(
        mirror.broken,
        Some(verkstead_render::Broken::HomeUnreachable),
        "nothing can be fetched from a machine that is not answering",
    );
    assert!(
        !mirror.device.as_ref().expect("it is A's").reachable,
        "and the row draws that machine exactly as the sidebar draws it",
    );
    assert_eq!(names(&rows), vec!["work"], "and it stays on the list");

    let conversation = b.drafting().await;
    b.picks(conversation, mirror.id).await;

    assert_eq!(
        b.starts(conversation).await,
        refused_over("HomeUnreachable"),
        "and Start names the machine to go and wake up",
    );
}

/// A Conversation already paired with a Profile that has since broken reads as
/// paired with a broken one, rather than as one nothing was picked for.
///
/// Which is the whole of why none of the three takes a row out of a picker: a
/// Pairing made yesterday is still a Pairing, and a picker that had quietly
/// emptied itself would be a human looking for a Profile they know they chose.
#[tokio::test]
async fn a_pairing_against_a_broken_mirror_still_reads_as_a_pairing() {
    let (a, b, _holding) = linked_up().await;

    a.saves("work").await;

    let landed = b
        .profiles_saying(|rows| called(rows, "work").is_some())
        .await;
    let here = called(&landed, "work").expect("the mirror").id;

    let conversation = b.drafting().await;
    b.picks(conversation, here).await;

    // The account signs itself out at home, which is a mirror with no login to
    // be made of it.
    std::fs::remove_file(a.dir.path().join("accounts/work/.claude/.credentials.json")).unwrap();
    a.nudges.announce(Nudge::Profiles);

    b.profiles_saying(|rows| {
        called(rows, "work")
            .is_some_and(|row| row.broken == Some(verkstead_render::Broken::NoLoginAtHome))
    })
    .await;

    let opened: ConversationView = reading(
        &b.workbench,
        &format!("/api/ui/conversations/{conversation}"),
    )
    .await;

    let paired = opened
        .implementation_pairing
        .expect("the Pairing is still the Conversation's own");

    assert_eq!(paired.profile.id, here, "the same row it was paired with");
    assert_eq!(
        paired.profile.broken,
        Some(verkstead_render::Broken::NoLoginAtHome),
        "read as paired with a broken Profile rather than as unpaired",
    );
    assert!(
        !opened.ready_to_grill,
        "and the work is not ready to start, which is the pane saying it before \
         the press does",
    );
}
