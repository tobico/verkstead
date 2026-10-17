//! **Author mirrors**: every device of a cluster drawing one list of Authors,
//! each member's beside its own and marked with the device it is at home on
//! (ADR-0025, borrowing ADR-0020's *Shared Profiles*).
//!
//! **Two Verksteads, a real dial and a real stream**, as `tests/mirroring.rs`
//! stands them up for the Profiles: B reads A's own `/api/ui/authors` over the
//! Peer Listener, refreshed off the Nudge stream it holds to A, and every
//! assertion reads B's own `/api/ui/authors` — the list the Git switcher draws.
//! A's Authors are saved the way the Git section saves one, into A's own
//! `config.yaml`.

use std::net::SocketAddr;
use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::AuthorEntry;
use verkstead_schema::Nudge;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Environment, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::{Routers, open_database, routers_keeping_answering_devices_telling};
use verkstead_store::{AuthorMirror, Linking, forget_member, record_author_mirrors, record_member};

/// The device whose Authors are mirrored.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the device that mirrors them, which is where every assertion is made.
const B: &str = "0011223344556677889900aabbccddee";

/// And a third nobody links to: what it is for is the row A holds of *it*.
const C: &str = "ff11223344556677889900aabbccddee";

/// What A is written down as on B, which is what a mirror's device mark says.
const A_MACHINE: &str = "the-workstation";
const A_OS: &str = "macOS 15.1";

/// The one list, which is the path every assertion here reads.
const AUTHORS: &str = "/api/ui/authors";

/// The uuids of the Authors in this suite.
const WORK: &str = "6f1c2a52-7d0e-4c55-9a43-0d6b8f3e2a11";
const HOME: &str = "0b9e4d17-3c2a-4f8e-8d61-5a7c9e2b4f03";

const PATIENCE: std::time::Duration = std::time::Duration::from_millis(300);
const WAITING: std::time::Duration = std::time::Duration::from_secs(10);
const LOOKING: std::time::Duration = std::time::Duration::from_millis(50);
const SETTLING: std::time::Duration = std::time::Duration::from_secs(2);

/// One Verkstead: its store, its identity, its Nudges, and both routers over
/// its one state — keeping its settings files in its own directory.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    nudges: Nudges,
    cluster: Devices,
    workbench: Router,
    address: SocketAddr,
    dir: tempfile::TempDir,
}

impl Verkstead {
    async fn answering(id: &str) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
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
            Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422),
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
        } = routers_keeping_answering_devices_telling(
            pool.clone(),
            dir.path().to_owned(),
            cluster.clone(),
            nudges.clone(),
            machine(dir.path()),
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

    /// An Author saved the way the Git section saves one: its uuid, its
    /// fields, and the token left alone.
    async fn saves_author(&self, id: &str, display_name: &str, name: &str, email: &str) {
        let (status, said) = pressing(
            &self.workbench,
            "/api/ui/settings",
            Some(
                &serde_json::json!({
                    "git_author": {
                        "id": id,
                        "display_name": display_name,
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

    /// And taken away from the Git switcher.
    async fn removes_author(&self, id: &str) {
        let (status, said) = pressing(
            &self.workbench,
            &format!("/api/ui/authors/{id}/remove"),
            None,
        )
        .await;

        assert!(status.is_success(), "removing {id}: {status} {said}");
        assert_eq!(said, "\"Removed\"", "removing {id}");
    }

    async fn authors(&self) -> Vec<AuthorEntry> {
        let said = self.authors_as_written().await;

        serde_json::from_str(&said).unwrap_or_else(|why| panic!("{AUTHORS} answered {said}: {why}"))
    }

    /// The list as the bytes that cross the wire, for what must not be in them.
    async fn authors_as_written(&self) -> String {
        let answered = self
            .workbench
            .clone()
            .oneshot(Request::builder().uri(AUTHORS).body(Body::empty()).unwrap())
            .await
            .unwrap();

        let status = answered.status();
        let bytes = answered.into_body().collect().await.unwrap().to_bytes();
        let said = String::from_utf8_lossy(&bytes).into_owned();

        assert_eq!(status, StatusCode::OK, "GET {AUTHORS}: {said}");

        said
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
}

struct Holding(tokio::task::JoinHandle<()>);

impl Drop for Holding {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// The machine a device here is judged on, which nothing in this suite asks
/// anything of.
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

/// A and B, linked both ways, with B holding the streams.
async fn linked_up() -> (Verkstead, Verkstead, Holding) {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;

    let machine = platform::hostname();
    let os = platform::os_word(Platform::Linux, None);
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

    let holding = b.holding();

    (a, b, holding)
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

fn with<'a>(rows: &'a [AuthorEntry], id: &str) -> Option<&'a AuthorEntry> {
    rows.iter().find(|row| row.id == id)
}

/// An Author saved on A is on B's list within a Nudge, after B's own, marked
/// with A's name and OS word — and an edit at home redraws it there without a
/// restart.
#[tokio::test]
async fn a_members_authors_are_listed_here_marked_with_its_device() {
    let (a, b, _holding) = linked_up().await;

    b.saves_author(HOME, "", "Ada Lovelace", "ada@home.example")
        .await;
    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;

    let rows = b.authors_saying(|rows| with(rows, WORK).is_some()).await;

    assert_eq!(
        rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        vec![HOME, WORK],
        "this device's own first, then the member's, as one list",
    );
    assert_eq!(rows[0].device, None, "B's own carries no mark");

    let mirror = with(&rows, WORK).unwrap();
    let device = mirror.device.as_ref().expect("a mirror names its home");

    assert_eq!(device.id.as_deref(), Some(A));
    assert_eq!(device.name, A_MACHINE);
    assert_eq!(device.os.as_deref(), Some(A_OS));
    assert_eq!(mirror.display_name, "Work");
    assert_eq!(mirror.email, "ada@work.example");
    assert!(!mirror.holds_token);

    // And an edit at home reaches B off the Nudge the save announces.
    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@elsewhere.example")
        .await;

    b.authors_saying(|rows| {
        with(rows, WORK).is_some_and(|row| row.email == "ada@elsewhere.example")
    })
    .await;
}

/// What a member answers says whether a token is held and nothing else of it,
/// and a member's token never reaches this device through the list.
#[tokio::test]
async fn the_list_carries_no_token_nor_any_part_of_one() {
    let (a, b, _holding) = linked_up().await;

    std::fs::write(
        a.dir.path().join("secrets.yaml"),
        format!("author_tokens:\n  {WORK}: ghp_averysecrettokenWXYZ\n"),
    )
    .unwrap();

    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;

    let rows = b.authors_saying(|rows| with(rows, WORK).is_some()).await;

    assert!(with(&rows, WORK).unwrap().holds_token);

    for said in [a.authors_as_written().await, b.authors_as_written().await] {
        assert!(!said.contains("ghp_"), "no token in {said}");
        assert!(!said.contains("WXYZ"), "nor its last four in {said}");
    }
}

/// What a member is mirroring is not taken in: B takes A's own Authors and
/// none of the ones A holds of C.
#[tokio::test]
async fn a_members_own_authors_are_taken_and_its_mirrors_are_not() {
    let (a, b, _holding) = linked_up().await;

    record_author_mirrors(
        &a.pool,
        C,
        &[AuthorMirror {
            id: HOME.to_owned(),
            device: C.to_owned(),
            display_name: String::new(),
            name: "The Third".to_owned(),
            email: "third@example.com".to_owned(),
            holds_token: false,
        }],
    )
    .await
    .unwrap();

    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;

    assert!(
        with(&a.authors().await, HOME).is_some(),
        "A draws the one it mirrors of C",
    );

    b.authors_saying(|rows| with(rows, WORK).is_some()).await;
    tokio::time::sleep(SETTLING).await;

    assert_eq!(
        b.authors()
            .await
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec![WORK],
        "A's own Author, and nothing A is itself mirroring",
    );
}

/// An Author removed at home is gone from B on the next refresh.
#[tokio::test]
async fn a_removal_at_home_takes_the_mirror() {
    let (a, b, _holding) = linked_up().await;

    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;
    a.saves_author(HOME, "Home", "Ada Lovelace", "ada@home.example")
        .await;

    b.authors_saying(|rows| with(rows, WORK).is_some() && with(rows, HOME).is_some())
        .await;

    a.removes_author(WORK).await;

    let rows = b.authors_saying(|rows| with(rows, WORK).is_none()).await;

    assert!(with(&rows, HOME).is_some(), "the one still at home stays");
}

/// The mirrors of a device that is no longer a member go.
#[tokio::test]
async fn the_mirrors_of_an_unlinked_device_go() {
    let (a, b, _holding) = linked_up().await;

    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;
    b.authors_saying(|rows| with(rows, WORK).is_some()).await;

    forget_member(&b.pool, A).await.unwrap();
    b.nudges.announce(Nudge::Devices);

    b.authors_saying(|rows| rows.is_empty()).await;
}

/// A member that is not answering keeps the rows it last gave.
#[tokio::test]
async fn a_member_that_stops_answering_keeps_its_rows() {
    let b = Verkstead::answering(B).await;

    let away = tempfile::tempdir().unwrap();
    let asleep = Device::stated(away.path(), A).unwrap();

    b.linked_to(&asleep, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    record_author_mirrors(
        &b.pool,
        A,
        &[AuthorMirror {
            id: WORK.to_owned(),
            device: A.to_owned(),
            display_name: "Work".to_owned(),
            name: "Ada Lovelace".to_owned(),
            email: "ada@work.example".to_owned(),
            holds_token: true,
        }],
    )
    .await
    .unwrap();

    let _holding = b.holding();

    b.nudges.announce(Nudge::Devices);

    tokio::time::sleep(SETTLING).await;

    let rows = b.authors().await;

    assert_eq!(rows.len(), 1, "the row A last gave stands: {rows:#?}");
    assert!(
        !rows[0].device.as_ref().expect("it is still A's").reachable,
        "drawn as the machine that did not answer",
    );
}
