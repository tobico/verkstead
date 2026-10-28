//! **Replicated records across a cluster**: every device holding every
//! Author, each serving its whole table to its members and pulling theirs
//! (ADR-0028).
//!
//! **Real Verksteads, real dials and real streams**, as `tests/mirroring.rs`
//! stands them up for the Profiles: each serves `GET
//! /api/peer/v1/records/authors` over its Peer Listener behind the Member
//! Gate, and pulls each member's on the schedule — at the start, on a member's
//! Authors Nudge, when its Nudge stream is taken up and when the membership
//! moves. Every assertion reads a device's own `/api/ui/authors`, the list the
//! Git switcher draws, and every Author is written the way the Git section
//! writes one.
//!
//! **A device switched off** is its Peer Listener stopped and its streams let
//! go; **switched on** is a new server over the same Data Directory, which
//! mints a new certificate, so the devices it is linked to write it down again.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{
    AuthorEntry, AuthorRemoved, DevicesView, PointingRepo, Registered, RepoEntry, UnrepointedRepo,
    Unrepointing,
};
use verkstead_schema::Nudge;
use verkstead_server::device::reading::{RUNNING, Reading};
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Environment, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::settings::Settings;
use verkstead_server::{
    Routers, open_database, replicating, routers_keeping_answering_devices_telling,
};
use verkstead_store::{Linking, forget_member, record_member};

const A: &str = "aa00bb11cc22dd33ee44ff5566778899";
const B: &str = "0011223344556677889900aabbccddee";
const C: &str = "ff11223344556677889900aabbccddee";

/// What A calls itself, and what every row for it says, so that a member
/// taking up its stream finds that row as it stands.
const A_MACHINE: &str = "the-workstation";
const A_OS: &str = "macOS 15.1";

/// The one list, which is the path every assertion here reads.
const AUTHORS: &str = "/api/ui/authors";

/// And where a member's records are pulled from.
const RECORDS: &str = "/api/peer/v1/records/authors";

/// And where an Author's token is copied from.
const TOKENS: &str = "/api/peer/v1/authors/";

const WORK: &str = "6f1c2a52-7d0e-4c55-9a43-0d6b8f3e2a11";
const PERSONAL: &str = "0b7e5f3a-2c41-4d8e-b6a9-5e3f1c7d9a20";

const PATIENCE: std::time::Duration = std::time::Duration::from_millis(300);
const WAITING: std::time::Duration = std::time::Duration::from_secs(10);
const LOOKING: std::time::Duration = std::time::Duration::from_millis(50);

/// One Verkstead: its store, its identity, its Nudges, and both routers over
/// its one state — keeping its settings files in its own directory.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    nudges: Nudges,
    cluster: Devices,
    reading: Reading,
    workbench: Router,

    /// The namespace its Peer Listener serves a member, asked in process.
    over_the_link: Router,

    address: SocketAddr,

    /// What its records route answers in place of the records while it is
    /// set: `404`, a member too old to have the route, or `503`, a device the
    /// test holds apart from the others. `0` answers.
    answering: Arc<AtomicU16>,

    /// And what its token route answers in place of a token, the same way.
    tokens_answering: Arc<AtomicU16>,

    /// The Peer Listener, stopped when the device is switched off.
    serving: tokio::task::JoinHandle<anyhow::Result<()>>,

    dir: tempfile::TempDir,
}

impl Verkstead {
    async fn answering(id: &str) -> Verkstead {
        Verkstead::standing(id, tempfile::tempdir().unwrap()).await
    }

    /// A server over the Data Directory `dir`, which is a start: the Authors
    /// taken up and held as the serve does, and every member pulled.
    async fn standing(id: &str, dir: tempfile::TempDir) -> Verkstead {
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        replicating::authors::taken_up(&pool, &Settings::in_data_dir(dir.path()), id)
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();
        let members = Members::recorded(pool.clone());
        let nudges = Nudges::new();

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");
        let address = listener.address();

        let reading = Reading::advertising(
            Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422),
            Platform::Linux,
            None,
            vec![format!("127.0.0.1:{}", address.port())],
        );

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
            dir.path().to_owned(),
            cluster.clone(),
            nudges.clone(),
            machine(dir.path()),
        );

        let answering = Arc::new(AtomicU16::new(0));
        let tokens_answering = Arc::new(AtomicU16::new(0));
        let asked_in_process = over_the_link.clone();

        let over_the_link = over_the_link.layer(axum::middleware::from_fn({
            let answering = answering.clone();
            let tokens_answering = tokens_answering.clone();

            move |asked: Request<Body>, next: axum::middleware::Next| {
                let path = asked.uri().path();
                let instead = match path {
                    RECORDS => answering.load(Ordering::SeqCst),
                    _ if path.starts_with(TOKENS) => tokens_answering.load(Ordering::SeqCst),
                    _ => 0,
                };

                async move {
                    match StatusCode::from_u16(instead) {
                        Ok(status) => axum::response::IntoResponse::into_response(status),
                        Err(_) => next.run(asked).await,
                    }
                }
            }
        }));

        let serving = tokio::spawn(listener.serving(peer::router(
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
            reading,
            workbench,
            over_the_link: asked_in_process,
            address,
            answering,
            tokens_answering,
            serving,
            dir,
        }
    }

    /// Switch it off, and back on over the same Data Directory.
    async fn switched_off(self) -> tempfile::TempDir {
        self.serving.abort();
        self.dir
    }

    /// Hold a Nudge stream to every member, which a running server spawns at
    /// the start.
    fn holding(&self) -> Holding {
        let cluster = self.cluster.clone();
        let nudges = self.nudges.clone();

        Holding(tokio::spawn(async move {
            cluster.stay_fresh(nudges).await;
        }))
    }

    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    /// Write `other` down as a member, as it is now.
    async fn links(&self, other: &Verkstead) {
        let (name, os) = match other.device.id() {
            A => (A_MACHINE.to_owned(), A_OS.to_owned()),
            _ => (
                platform::hostname(),
                platform::os_word(Platform::Linux, None),
            ),
        };

        record_member(
            &self.pool,
            &Linking {
                device: other.device.id().to_owned(),
                name,
                os,
                addresses: vec![other.at()],
                fingerprint: other.device.fingerprint().to_owned(),
                kept_by: None,
                version: Some(RUNNING.to_owned()),
                install: Some("Binary".to_owned()),
            },
        )
        .await
        .unwrap();
    }

    /// Answer the records route with `status` in place of the records, or
    /// answer it again with `None`.
    fn records_answer(&self, status: Option<StatusCode>) {
        self.answering
            .store(status.map_or(0, |status| status.as_u16()), Ordering::SeqCst);
    }

    /// Answer the token route with `status` in place of a token, or answer it
    /// again with `None`.
    fn tokens_answer(&self, status: Option<StatusCode>) {
        self.tokens_answering
            .store(status.map_or(0, |status| status.as_u16()), Ordering::SeqCst);
    }

    /// An Author saved the way the Git section saves one.
    async fn saves_author(&self, id: &str, name: &str, email: &str) {
        let (status, said) = pressing(
            &self.workbench,
            "/api/ui/settings",
            Some(
                &serde_json::json!({
                    "git_author": {
                        "id": id,
                        "display_name": "",
                        "name": name,
                        "email": email,
                    },
                    "github_token": "Keep",
                    "languages": [{ "name": "rust", "enabled": true, "size": "" }],
                    "cleanup": {
                        "trim": { "enabled": true, "days": "" },
                        "delete": { "enabled": false, "days": "" },
                    },
                    "at_once": { "roadmap_stages": "", "conversations": "" },
                    "conflict_resolution": "Merge",
                    "share_on_done": false,
                    "sandbox_binds": [],
                    "ignored_comments": "Keep",
                    "mcp_servers": "Keep",
                    "instructions": "",
                })
                .to_string(),
            ),
        )
        .await;

        assert!(status.is_success(), "saving {name}: {status} {said}");
    }

    /// And one Author edited alone, the token left as it is.
    async fn edits_author(&self, id: &str, name: &str, email: &str) {
        let (status, said) = pressing(
            &self.workbench,
            &format!("/api/ui/authors/{id}"),
            Some(
                &serde_json::json!({
                    "display_name": "",
                    "name": name,
                    "email": email,
                    "github_token": "Keep",
                })
                .to_string(),
            ),
        )
        .await;

        assert!(status.is_success(), "editing {id}: {status} {said}");
        assert!(said.contains("Saved"), "editing {id}: {said}");
    }

    /// And one Author's token set or cleared alone, the way the Git section
    /// sends `github_token`, its fields as they are.
    async fn tokens_author(&self, id: &str, token: serde_json::Value) {
        let row = with(&self.authors().await, id)
            .cloned()
            .expect("the Author is in the list");

        let (status, said) = pressing(
            &self.workbench,
            &format!("/api/ui/authors/{id}"),
            Some(
                &serde_json::json!({
                    "display_name": row.display_name,
                    "name": row.name,
                    "email": row.email,
                    "github_token": token,
                })
                .to_string(),
            ),
        )
        .await;

        assert!(status.is_success(), "tokening {id}: {status} {said}");
        assert!(said.contains("Saved"), "tokening {id}: {said}");
    }

    async fn sets_token(&self, id: &str, token: &str) {
        self.tokens_author(id, serde_json::json!({ "Set": { "token": token } }))
            .await;
    }

    async fn clears_token(&self, id: &str) {
        self.tokens_author(id, serde_json::json!("Clear")).await;
    }

    fn settings(&self) -> Settings {
        Settings::in_data_dir(self.dir.path())
    }

    /// The token this device's `secrets.yaml` holds for the Author `id`.
    fn token_here(&self, id: &str) -> Option<String> {
        self.settings()
            .secrets()
            .author_token(id)
            .map(str::to_owned)
    }

    /// Wait for this device's `secrets.yaml` to hold `token` for the Author
    /// `id` — or none, where `token` is `None`.
    async fn token_saying(&self, id: &str, token: Option<&str>) {
        let waited = tokio::time::timeout(WAITING, async {
            while self.token_here(id).as_deref() != token {
                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| {
            panic!(
                "the token never said {token:?}: it said {:?}",
                self.token_here(id)
            )
        });
    }

    async fn removes_author(&self, id: &str) {
        let (status, said) = pressing(
            &self.workbench,
            &format!("/api/ui/authors/{id}/remove"),
            None,
        )
        .await;

        assert!(status.is_success(), "removing {id}: {status} {said}");
        assert!(said.contains("\"Removed\""), "removing {id}: {said}");
    }

    /// The removal pressed, and what it answered.
    async fn removing_author(&self, id: &str) -> AuthorRemoved {
        let (status, said) = pressing(
            &self.workbench,
            &format!("/api/ui/authors/{id}/remove"),
            None,
        )
        .await;

        assert!(status.is_success(), "removing {id}: {status} {said}");
        serde_json::from_str(&said).unwrap()
    }

    /// **Repoint them all to…** pressed on the refusal, and what it answered.
    async fn repointing_all(&self, id: &str, to: &str) -> AuthorRemoved {
        let (status, said) = pressing(
            &self.workbench,
            &format!("/api/ui/authors/{id}/repoint"),
            Some(&serde_json::json!({ "author": to }).to_string()),
        )
        .await;

        assert!(status.is_success(), "repointing {id}: {status} {said}");
        serde_json::from_str(&said).unwrap()
    }

    /// A repository made under this device's directory and registered as
    /// the Author `author`, by its id.
    async fn registers(&self, name: &str, author: &str) -> i64 {
        let path = self.dir.path().join("src").join(name);
        std::fs::create_dir_all(&path).unwrap();

        for args in [
            &["init", "--initial-branch", "main"][..],
            &[
                "-c",
                "user.email=t@verkstead.invalid",
                "-c",
                "user.name=T",
                "commit",
                "--allow-empty",
                "-m",
                "first",
            ],
        ] {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .expect("git should be on the PATH for these tests");
            assert!(status.success(), "git {args:?}");
        }

        let (status, said) = pressing(
            &self.workbench,
            "/api/ui/repos",
            Some(&serde_json::json!({ "path": path, "author": author }).to_string()),
        )
        .await;

        assert!(status.is_success(), "registering {name}: {status} {said}");

        match serde_json::from_str(&said).unwrap() {
            Registered::Added(repo) => repo.id,
            refused => panic!("registering {name}: {refused:?}"),
        }
    }

    /// The Author the Repo `id` records.
    async fn repo_author(&self, id: i64) -> Option<String> {
        let repos: Vec<RepoEntry> = reading(&self.workbench, "/api/ui/repos").await;

        repos
            .into_iter()
            .find(|repo| repo.id == id)
            .expect("the Repo is registered")
            .author
    }

    async fn authors(&self) -> Vec<AuthorEntry> {
        reading(&self.workbench, AUTHORS).await
    }

    /// The list once it says what the test is waiting for, or a panic saying
    /// what it said instead.
    async fn authors_saying(&self, what: impl Fn(&[AuthorEntry]) -> bool) -> Vec<AuthorEntry> {
        let mut last = Vec::new();

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                last = self.authors().await;

                if what(&last) {
                    return last.clone();
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("the Authors never said it: {last:#?}"))
    }

    /// The stamps this device holds for the Author `id`: `updated_at` and the
    /// deletion stamp.
    async fn stamps_of(&self, id: &str) -> (i64, Option<i64>) {
        sqlx::query_as(
            "SELECT updated_at, deleted_at FROM replicated WHERE kind = 'authors' AND uuid = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    /// The line this device's Devices section draws under `member`'s row, once
    /// it says what the test is waiting for.
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

        waited.unwrap_or_else(|_| panic!("the row never said {what:?}: it said {last:?}"));
    }
}

struct Holding(tokio::task::JoinHandle<()>);

impl Drop for Holding {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn machine(dir: &Path) -> Machine {
    Machine::stated(
        Platform::Linux,
        dir.as_os_str().to_owned(),
        dir.as_os_str().to_owned(),
        None,
        None,
        &Environment {
            home: Some(dir.to_owned()),
            ..Environment::default()
        },
    )
}

/// `one` and `other`, linked both ways.
async fn linked(one: &Verkstead, other: &Verkstead) {
    one.links(other).await;
    other.links(one).await;
}

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

fn with<'a>(rows: &'a [AuthorEntry], id: &str) -> Option<&'a AuthorEntry> {
    rows.iter().find(|row| row.id == id)
}

fn emailed(rows: &[AuthorEntry], id: &str, email: &str) -> bool {
    with(rows, id).is_some_and(|row| row.email == email)
}

/// The demonstration: an Author made on A is on B after the Nudge, the same
/// uuid and no device mark; A switched off, the Author edited on B, A switched
/// on — and A shows the edit after its start pull.
#[tokio::test]
async fn an_author_made_on_one_device_is_on_the_other_and_an_edit_made_while_it_was_off_lands() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    linked(&a, &b).await;
    let _b_holding = b.holding();

    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;

    b.authors_saying(|rows| emailed(rows, WORK, "ada@work.example"))
        .await;

    let dir = a.switched_off().await;

    b.edits_author(WORK, "Ada Lovelace", "ada@elsewhere.example")
        .await;

    let a = Verkstead::standing(A, dir).await;
    b.links(&a).await;

    a.authors_saying(|rows| emailed(rows, WORK, "ada@elsewhere.example"))
        .await;
}

/// B taken down while A makes an Author, then brought back, has it after its
/// start pull.
#[tokio::test]
async fn a_device_that_was_off_catches_up_at_its_start() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    linked(&a, &b).await;

    let dir = b.switched_off().await;

    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;

    let b = Verkstead::standing(B, dir).await;
    a.links(&b).await;

    b.authors_saying(|rows| emailed(rows, WORK, "ada@work.example"))
        .await;
}

/// An edit on B, whose clock is behind A's, of a record A last wrote is
/// stamped past A's write — and A takes it on the next pull rather than
/// keeping its own.
#[tokio::test]
async fn an_edit_on_a_clock_behind_is_stamped_past_what_it_was_made_on_and_taken() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    linked(&a, &b).await;
    let _a_holding = a.holding();
    let _b_holding = b.holding();

    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;

    // A's clock an hour ahead of B's, which is what its write was stamped by.
    sqlx::query("UPDATE replicated SET updated_at = updated_at + 3600000 WHERE uuid = ?")
        .bind(WORK)
        .execute(&a.pool)
        .await
        .unwrap();
    let (ahead, _) = a.stamps_of(WORK).await;
    a.nudges.announce(Nudge::Authors);

    b.authors_saying(|rows| emailed(rows, WORK, "ada@work.example"))
        .await;
    tokio::time::timeout(WAITING, async {
        while b.stamps_of(WORK).await.0 != ahead {
            tokio::time::sleep(LOOKING).await;
        }
    })
    .await
    .expect("B holds A's stamp");

    b.edits_author(WORK, "Ada Lovelace", "ada@behind.example")
        .await;
    assert_eq!(b.stamps_of(WORK).await.0, ahead + 1, "one past A's write");

    a.authors_saying(|rows| emailed(rows, WORK, "ada@behind.example"))
        .await;
    assert_eq!(a.stamps_of(WORK).await.0, ahead + 1);
}

/// An Author removed on A and edited on B while the two were apart stays
/// removed on both once they meet — the edit stamped later than the removal
/// all the same.
#[tokio::test]
async fn an_author_removed_on_one_and_edited_on_the_other_stays_removed() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    linked(&a, &b).await;
    let _a_holding = a.holding();
    let _b_holding = b.holding();

    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;
    b.authors_saying(|rows| with(rows, WORK).is_some()).await;

    a.records_answer(Some(StatusCode::SERVICE_UNAVAILABLE));
    b.records_answer(Some(StatusCode::SERVICE_UNAVAILABLE));

    a.removes_author(WORK).await;
    b.edits_author(WORK, "Ada Lovelace", "ada@apart.example")
        .await;

    let (removed, _) = a.stamps_of(WORK).await;
    let (edited, _) = b.stamps_of(WORK).await;
    assert!(edited > removed, "the edit is the later of the two");

    a.records_answer(None);
    b.records_answer(None);
    a.nudges.announce(Nudge::Devices);
    b.nudges.announce(Nudge::Devices);

    b.authors_saying(|rows| with(rows, WORK).is_none()).await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    assert!(with(&a.authors().await, WORK).is_none());
    assert!(with(&b.authors().await, WORK).is_none());
    assert!(a.stamps_of(WORK).await.1.is_some());
    assert!(b.stamps_of(WORK).await.1.is_some());
}

/// An Author made on A reaches C through B after A is unlinked: B serves
/// every record it holds, whoever wrote it.
#[tokio::test]
async fn an_author_reaches_a_newcomer_through_whoever_holds_it() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    linked(&a, &b).await;
    let _b_holding = b.holding();

    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;
    b.authors_saying(|rows| with(rows, WORK).is_some()).await;

    forget_member(&b.pool, A).await.unwrap();
    let _gone = a.switched_off().await;

    let c = Verkstead::answering(C).await;
    linked(&b, &c).await;
    c.nudges.announce(Nudge::Devices);

    c.authors_saying(|rows| emailed(rows, WORK, "ada@work.example"))
        .await;
}

/// A fresh start on a database holding Author mirrors shows one list with the
/// same uuids and no device marks — and the real record replaces each mirror
/// on the first pull of the member that holds it.
#[tokio::test]
async fn the_author_mirrors_are_records_until_the_real_ones_are_pulled() {
    let a = Verkstead::answering(A).await;
    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;

    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    sqlx::query(
        "CREATE TABLE author_mirrors (
             id           TEXT PRIMARY KEY,
             home_device  TEXT NOT NULL,
             display_name TEXT NOT NULL,
             name         TEXT NOT NULL,
             email        TEXT NOT NULL,
             holds_token  INTEGER NOT NULL
         ) STRICT",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO author_mirrors VALUES (?, ?, '', 'Ada', 'ada@mirrored.example', 0)")
        .bind(WORK)
        .bind(A)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let b = Verkstead::standing(B, dir).await;

    let mirrored = b.authors().await;
    assert_eq!(
        mirrored
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        [WORK]
    );
    assert_eq!(mirrored[0].email, "ada@mirrored.example");

    let there: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE name = 'author_mirrors'")
            .fetch_optional(&b.pool)
            .await
            .unwrap();
    assert_eq!(there, None, "author_mirrors is gone");

    linked(&a, &b).await;
    b.nudges.announce(Nudge::Devices);

    b.authors_saying(|rows| emailed(rows, WORK, "ada@work.example"))
        .await;
}

/// A member too old to answer the route — through the Member Gate, and `404`
/// — leaves a trouble line on its row naming both releases, and its Authors
/// are absent; and the line goes once it answers.
#[tokio::test]
async fn a_member_too_old_for_the_route_is_said_on_its_row_with_both_releases() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    a.reading.now_running(Some("0.0.1"));
    a.records_answer(Some(StatusCode::NOT_FOUND));
    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;

    linked(&a, &b).await;
    let _b_holding = b.holding();

    b.trouble_on(
        A,
        Some(&format!(
            "{A_MACHINE}'s Authors couldn't be read. {A_MACHINE} is on 0.0.1 and this device \
             is on {RUNNING}. Update {A_MACHINE}."
        )),
    )
    .await;
    assert!(with(&b.authors().await, WORK).is_none());

    a.records_answer(None);
    b.nudges.announce(Nudge::Devices);

    b.trouble_on(A, None).await;
    b.authors_saying(|rows| with(rows, WORK).is_some()).await;
}

/// The route serves removed records beside live ones, and never a token.
#[tokio::test]
async fn the_route_serves_removed_records_and_no_token() {
    let a = Verkstead::answering(A).await;

    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;
    std::fs::write(
        a.dir.path().join("secrets.yaml"),
        format!("author_tokens:\n  {WORK}: ghp_secretvalue\n"),
    )
    .unwrap();
    a.edits_author(WORK, "Ada Lovelace", "ada@home.example")
        .await;
    a.saves_author("other", "Grace Hopper", "grace@work.example")
        .await;
    a.removes_author("other").await;

    let said: serde_json::Value = reading(&a.over_the_link, RECORDS).await;
    let said = said.to_string();

    assert!(!said.contains("ghp_"), "{said}");
    let records: Vec<serde_json::Value> = serde_json::from_str(&said).unwrap();
    assert_eq!(records.len(), 2, "{said}");
    assert!(records[1]["deleted_at"].is_i64(), "{said}");
}

/// A token set on B for an Author C made reaches A's `secrets.yaml` with C
/// switched off: it travels from whoever holds one. And once the copy has
/// landed, a Repo on A pointed at that Author is no longer awaiting it — its
/// sessions are handed the copy.
#[tokio::test]
async fn a_token_set_on_one_device_reaches_another_with_the_authors_maker_off() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;
    linked(&a, &b).await;
    linked(&a, &c).await;
    linked(&b, &c).await;
    let _a_holding = a.holding();
    let _b_holding = b.holding();

    c.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;
    c.nudges.announce(Nudge::Devices);
    a.authors_saying(|rows| with(rows, WORK).is_some()).await;
    b.authors_saying(|rows| with(rows, WORK).is_some()).await;

    let _off = c.switched_off().await;

    b.sets_token(WORK, "ghp_setonb").await;

    a.token_saying(WORK, Some("ghp_setonb")).await;

    assert_eq!(
        a.settings().config().author_missing(Some(WORK)),
        None,
        "a Repo pointed at it starts, handed the copy",
    );

    // And a rotation, which the copy follows.
    b.sets_token(WORK, "ghp_rotatedonb").await;
    a.token_saying(WORK, Some("ghp_rotatedonb")).await;
}

/// A token's record saying one is held, before any copy has reached a device,
/// is a token awaited there — and nothing else is.
#[tokio::test]
async fn a_token_held_elsewhere_is_awaited_until_its_copy_lands() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    linked(&a, &b).await;
    let _b_holding = b.holding();

    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;
    b.authors_saying(|rows| with(rows, WORK).is_some()).await;
    assert_eq!(b.settings().config().author_missing(Some(WORK)), None);

    // A's token route held back, so the record arrives and its token does not.
    a.tokens_answer(Some(StatusCode::SERVICE_UNAVAILABLE));
    a.sets_token(WORK, "ghp_seton_a").await;

    tokio::time::timeout(WAITING, async {
        while b.settings().config().author_missing(Some(WORK)).is_none() {
            tokio::time::sleep(LOOKING).await;
        }
    })
    .await
    .expect("the token is awaited on B");
    assert_eq!(b.token_here(WORK), None);

    // And once A answers, the next pull fetches it.
    a.tokens_answer(None);
    a.nudges.announce(Nudge::Authors);

    b.token_saying(WORK, Some("ghp_seton_a")).await;
    assert_eq!(b.settings().config().author_missing(Some(WORK)), None);
}

/// A token cleared on one device is taken out of every other device's
/// `secrets.yaml`, the device it was set on included; and an Author removed
/// takes every copy of its token with it.
#[tokio::test]
async fn a_token_cleared_or_its_author_removed_takes_every_copy_away() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    linked(&a, &b).await;
    let _a_holding = a.holding();
    let _b_holding = b.holding();

    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;
    a.sets_token(WORK, "ghp_seton_a").await;
    b.token_saying(WORK, Some("ghp_seton_a")).await;

    // Cleared on B, where it is a copy.
    b.clears_token(WORK).await;
    a.token_saying(WORK, None).await;
    assert_eq!(b.token_here(WORK), None);

    // An edit on a device holding no copy says nothing about the token.
    a.sets_token(WORK, "ghp_again").await;
    b.token_saying(WORK, Some("ghp_again")).await;
    std::fs::write(b.dir.path().join("secrets.yaml"), "").unwrap();
    b.edits_author(WORK, "Ada Lovelace", "ada@edited.example")
        .await;
    a.authors_saying(|rows| emailed(rows, WORK, "ada@edited.example"))
        .await;
    assert_eq!(a.token_here(WORK).as_deref(), Some("ghp_again"));

    // And removed on A.
    a.removes_author(WORK).await;
    b.token_saying(WORK, None).await;
    assert_eq!(a.token_here(WORK), None);
}

/// The token route answers for any Author this device holds a record of — a
/// copy it fetched included — `null` where it holds no token, and `404` for a
/// uuid it holds no live record of.
#[tokio::test]
async fn the_token_route_answers_from_whoever_holds_one() {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    let c = Verkstead::answering(C).await;
    linked(&a, &b).await;
    linked(&b, &c).await;
    let _b_holding = b.holding();

    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;
    a.saves_author("other", "Grace Hopper", "grace@work.example")
        .await;
    a.sets_token(WORK, "ghp_seton_a").await;
    b.token_saying(WORK, Some("ghp_seton_a")).await;

    let asked = |id: &'static str| replicating::authors::token_at(Some(&c.cluster), B, id);

    assert_eq!(
        asked(WORK).await.unwrap().as_deref(),
        Some("ghp_seton_a"),
        "B's copy"
    );
    assert_eq!(asked("other").await.unwrap(), None, "no token held");
    let refused = asked("no-such-uuid").await.unwrap_err();
    assert!(refused.saying.contains("404"), "{}", refused.saying);
}

/// A Repo of `device`'s, as a removal names one.
fn on(name: &str, device: Option<&str>) -> PointingRepo {
    PointingRepo {
        name: name.to_owned(),
        device: device.map(str::to_owned),
    }
}

/// A and B holding WORK and PERSONAL, made on A, each pulled by B.
async fn two_authors_on_both() -> (Verkstead, Verkstead, Holding, Holding) {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;
    linked(&a, &b).await;
    let a_holding = a.holding();
    let b_holding = b.holding();

    a.saves_author(WORK, "Ada Lovelace", "ada@work.example")
        .await;
    a.saves_author(PERSONAL, "Ada Lovelace", "ada@home.example")
        .await;
    b.authors_saying(|rows| with(rows, WORK).is_some() && with(rows, PERSONAL).is_some())
        .await;

    (a, b, a_holding, b_holding)
}

/// Removing an Author a member's Repo points at is refused naming that Repo
/// and the member's device, beside this device's own — and a member not
/// answering is skipped and said, blocking nothing once nothing else points.
#[tokio::test]
async fn removing_an_author_a_members_repo_points_at_names_the_repo_and_its_device() {
    let (a, b, _a_holding, _b_holding) = two_authors_on_both().await;

    let theirs = a.registers("widgets", WORK).await;
    b.registers("gadgets", WORK).await;

    let c = Verkstead::answering(C).await;
    linked(&b, &c).await;
    let _gone = c.switched_off().await;
    let not_answering = platform::hostname();

    assert_eq!(
        b.removing_author(WORK).await,
        AuthorRemoved::InUse {
            repos: vec![on("gadgets", None), on("widgets", Some(A_MACHINE))],
            unasked: vec![not_answering.clone()],
        },
    );
    assert!(
        with(&b.authors().await, WORK).is_some(),
        "nothing was removed"
    );

    // Nothing points at PERSONAL anywhere answering, so the silent member
    // does not stop it.
    assert_eq!(
        b.removing_author(PERSONAL).await,
        AuthorRemoved::Removed {
            unasked: vec![not_answering],
        },
    );
    a.authors_saying(|rows| with(rows, PERSONAL).is_none())
        .await;
    assert_eq!(a.repo_author(theirs).await.as_deref(), Some(WORK));
}

/// **Repoint them all to…** repoints this device's Repo and the member's from
/// here, and then the Author is removed on every device.
#[tokio::test]
async fn repointing_them_all_repoints_every_devices_repos_and_then_removes_the_author() {
    let (a, b, _a_holding, _b_holding) = two_authors_on_both().await;

    let theirs = a.registers("widgets", WORK).await;
    let ours = b.registers("gadgets", WORK).await;

    assert_eq!(
        b.repointing_all(WORK, PERSONAL).await,
        AuthorRemoved::Removed { unasked: vec![] },
    );

    assert_eq!(a.repo_author(theirs).await.as_deref(), Some(PERSONAL));
    assert_eq!(b.repo_author(ours).await.as_deref(), Some(PERSONAL));
    assert!(with(&b.authors().await, WORK).is_none());
    a.authors_saying(|rows| with(rows, WORK).is_none() && with(rows, PERSONAL).is_some())
        .await;
}

/// A member's Repo with a live Conversation on it refuses the repoint by
/// name, and the Author stays; this device's Repo, under no live work, is
/// repointed all the same.
#[tokio::test]
async fn a_repo_with_a_live_conversation_refuses_repointing_them_all_and_the_author_stays() {
    let (a, b, _a_holding, _b_holding) = two_authors_on_both().await;

    let theirs = a.registers("widgets", WORK).await;
    let ours = b.registers("gadgets", WORK).await;

    let going = verkstead_store::start_conversation(&a.pool, theirs, "rate-limiting", A, None)
        .await
        .unwrap()
        .unwrap();
    verkstead_store::set_state(&a.pool, going, verkstead_store::Lifecycle::Grilling)
        .await
        .unwrap();

    assert_eq!(
        b.repointing_all(WORK, PERSONAL).await,
        AuthorRemoved::Unrepointed {
            repos: vec![UnrepointedRepo {
                repo: on("widgets", Some(A_MACHINE)),
                why: Unrepointing::InUse {
                    on: vec!["rate-limiting".to_owned()],
                    beside: vec![],
                },
            }],
            unasked: vec![],
        },
    );

    assert_eq!(a.repo_author(theirs).await.as_deref(), Some(WORK));
    assert_eq!(b.repo_author(ours).await.as_deref(), Some(PERSONAL));
    assert!(with(&b.authors().await, WORK).is_some(), "the Author stays");
}
