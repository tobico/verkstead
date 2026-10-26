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
use verkstead_server::device::reading::{RUNNING, Reading};
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

/// What A is written down as on B, which is what a mirror's device mark says —
/// and what A says of itself, so that a take-up of its stream finds it so.
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
                kept_by: None,
                version: Some(RUNNING.to_owned()),
                install: Some("Binary".to_owned()),
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

/// A git repository with one commit on `main`, which is all a registration
/// asks of one.
fn repository(path: &Path) {
    std::fs::create_dir_all(path).unwrap();

    for args in [
        &["init", "--quiet", "--initial-branch", "main"][..],
        &[
            "-c",
            "user.name=Verkstead Test",
            "-c",
            "user.email=test@verkstead.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "first",
        ],
    ] {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(path)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }
}

/// A member's Author, once the refresher has written its mirror down, is one
/// a Repo here can be opened as and repointed at — and once it has gone from
/// home, one it can be repointed at no longer.
#[tokio::test]
async fn a_repo_here_can_be_pointed_at_a_members_author() {
    let (a, b, _holding) = linked_up().await;

    b.saves_author(HOME, "", "Ada Lovelace", "ada@home.example")
        .await;
    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;
    b.authors_saying(|rows| with(rows, WORK).is_some()).await;

    let elsewhere = tempfile::tempdir().unwrap();
    let path = elsewhere.path().join("verkstead");
    repository(&path);

    let (status, said) = pressing(
        &b.workbench,
        "/api/ui/repos",
        Some(&serde_json::json!({ "path": path, "author": WORK }).to_string()),
    )
    .await;
    assert!(status.is_success(), "{status} {said}");

    let registered: verkstead_render::Registered = serde_json::from_str(&said).unwrap();
    let verkstead_render::Registered::Added(repo) = registered else {
        panic!("opened as the mirror: {said}");
    };
    assert_eq!(repo.author.as_deref(), Some(WORK));

    let repoint = |author: &'static str| {
        let workbench = b.workbench.clone();
        let id = repo.id;

        async move {
            let (_, said) = pressing(
                &workbench,
                &format!("/api/ui/repos/{id}/author"),
                Some(&serde_json::json!({ "author": author }).to_string()),
            )
            .await;

            serde_json::from_str::<verkstead_render::Repointed>(&said).unwrap()
        }
    };

    // Asked until it says so, the list read above and the Authors a press
    // resolves out of being written one after the other.
    let repointed_until = |author: &'static str, what: fn(&verkstead_render::Repointed) -> bool| {
        let repoint = &repoint;

        async move {
            let mut last = None;

            let waited = tokio::time::timeout(WAITING, async {
                loop {
                    let said = repoint(author).await;

                    if what(&said) {
                        return;
                    }

                    last = Some(said);
                    tokio::time::sleep(LOOKING).await;
                }
            })
            .await;

            waited.unwrap_or_else(|_| panic!("the repoint onto {author} said {last:?}"));
        }
    };

    assert!(matches!(
        repoint(HOME).await,
        verkstead_render::Repointed::Repointed(_)
    ));
    repointed_until(WORK, |said| {
        matches!(said, verkstead_render::Repointed::Repointed(_))
    })
    .await;

    a.removes_author(WORK).await;

    repointed_until(WORK, |said| {
        *said == verkstead_render::Repointed::NoSuchAuthor
    })
    .await;
}

/// What `secrets.yaml` in `dir` holds, as text.
fn secrets_in(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("secrets.yaml")).unwrap_or_default()
}

/// A secret of B's own that no copy may move: what every write of a copy has
/// to leave as it was.
const B_OWN: &str = "session_account_password: Vk1-hunter2\n";

impl Verkstead {
    /// The token one of `device`'s Authors has there, asked over the link the
    /// way the refresher asks it.
    async fn asks_token_of(&self, device: &str, id: &str) -> Result<Option<String>, String> {
        verkstead_server::mirroring::authors::token_at(Some(&self.cluster), device, id)
            .await
            .map_err(|refused| refused.saying)
    }

    /// Write `secrets.yaml` and tell the cluster, the way a token saved on the
    /// Git section does.
    fn holds_tokens(&self, yaml: &str) {
        std::fs::write(self.dir.path().join("secrets.yaml"), yaml).unwrap();
        self.nudges.announce(Nudge::Authors);
    }

    /// `secrets.yaml` once it says what the test is waiting for.
    async fn secrets_saying(&self, what: impl Fn(&str) -> bool) -> String {
        let mut last = String::new();

        let waited = tokio::time::timeout(WAITING, async {
            loop {
                last = secrets_in(self.dir.path());

                if what(&last) {
                    return last.clone();
                }

                tokio::time::sleep(LOOKING).await;
            }
        })
        .await;

        waited.unwrap_or_else(|_| panic!("secrets.yaml never said it:\n{last}"))
    }
}

/// A member's Author's token is copied into this device's `secrets.yaml` under
/// its uuid, beside what was there; a rotation at home is the copy after the
/// next refresh; and with the home not answering the copy stands.
#[tokio::test]
async fn a_members_token_is_copied_here_and_follows_a_rotation() {
    let (a, b, holding) = linked_up().await;

    std::fs::write(b.dir.path().join("secrets.yaml"), B_OWN).unwrap();

    a.holds_tokens(&format!(
        "author_tokens:\n  {WORK}: ghp_thefirst\n  {HOME}: ghp_theother\n"
    ));
    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;
    a.saves_author(HOME, "Home", "Ada Lovelace", "ada@home.example")
        .await;

    let said = b
        .secrets_saying(|said| said.contains("ghp_thefirst") && said.contains("ghp_theother"))
        .await;
    assert!(said.contains(&format!("{WORK}: ghp_thefirst")), "{said}");
    assert!(said.contains("Vk1-hunter2"), "B's own stays: {said}");

    a.holds_tokens(&format!(
        "author_tokens:\n  {WORK}: ghp_thesecond\n  {HOME}: ghp_theother\n"
    ));

    let said = b
        .secrets_saying(|said| said.contains("ghp_thesecond"))
        .await;
    assert!(
        !said.contains("ghp_thefirst"),
        "the old one is gone: {said}"
    );
    assert!(said.contains("Vk1-hunter2"), "B's own stays: {said}");

    // And with A gone quiet, a refresh leaves the copy where it is.
    drop(holding);
    b.linked_to(&a.device, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;
    b.nudges.announce(Nudge::Devices);
    tokio::time::sleep(SETTLING).await;

    assert!(
        secrets_in(b.dir.path()).contains("ghp_thesecond"),
        "the copy stands with its home quiet",
    );
}

/// Clearing the token at home, removing the Author at home, and unlinking the
/// device each take the copy out, and nothing else of B's file moves.
#[tokio::test]
async fn the_copy_goes_with_the_token_the_author_and_the_device() {
    let (a, b, _holding) = linked_up().await;

    std::fs::write(b.dir.path().join("secrets.yaml"), B_OWN).unwrap();

    a.holds_tokens(&format!(
        "author_tokens:\n  {WORK}: ghp_thework\n  {HOME}: ghp_thehome\n"
    ));
    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;
    a.saves_author(HOME, "Home", "Ada Lovelace", "ada@home.example")
        .await;

    b.secrets_saying(|said| said.contains("ghp_thework") && said.contains("ghp_thehome"))
        .await;

    // Cleared at home.
    a.holds_tokens(&format!("author_tokens:\n  {HOME}: ghp_thehome\n"));

    let said = b.secrets_saying(|said| !said.contains("ghp_thework")).await;
    assert!(
        said.contains("ghp_thehome") && said.contains("Vk1-hunter2"),
        "{said}"
    );

    // Removed at home.
    a.removes_author(HOME).await;

    let said = b.secrets_saying(|said| !said.contains("ghp_thehome")).await;
    assert!(said.contains("Vk1-hunter2"), "{said}");

    // And unlinked, with a fresh token to lose.
    a.holds_tokens(&format!("author_tokens:\n  {WORK}: ghp_again\n"));
    b.secrets_saying(|said| said.contains("ghp_again")).await;

    forget_member(&b.pool, A).await.unwrap();
    b.nudges.announce(Nudge::Devices);

    let said = b.secrets_saying(|said| !said.contains("ghp_again")).await;
    assert_eq!(said, B_OWN, "every other secret as it was");
}

/// A `secrets.yaml` here that will not parse is left exactly as it is by the
/// copy, rather than written over with nothing but the copy in it — and the
/// copy arrives once the file is put right.
#[tokio::test]
async fn a_secrets_file_that_wont_parse_is_left_alone_by_the_copy() {
    let (a, b, _holding) = linked_up().await;

    let broken = format!("{B_OWN}author_tokens: [oh\n");
    std::fs::write(b.dir.path().join("secrets.yaml"), &broken).unwrap();

    a.holds_tokens(&format!("author_tokens:\n  {WORK}: ghp_thework\n"));
    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;

    b.authors_saying(|rows| with(rows, WORK).is_some_and(|row| row.holds_token))
        .await;
    b.nudges.announce(Nudge::Devices);
    tokio::time::sleep(SETTLING).await;

    assert_eq!(
        secrets_in(b.dir.path()),
        broken,
        "the human's file, typo and all"
    );

    std::fs::write(b.dir.path().join("secrets.yaml"), B_OWN).unwrap();
    a.nudges.announce(Nudge::Authors);

    let said = b.secrets_saying(|said| said.contains("ghp_thework")).await;
    assert!(said.contains("Vk1-hunter2"), "B's own stays: {said}");
}

/// The token call hands over only the answering device's own Authors' tokens,
/// and only to a member.
#[tokio::test]
async fn the_token_call_refuses_a_mirror_an_unknown_uuid_and_a_stranger() {
    let (a, b, _holding) = linked_up().await;

    a.holds_tokens(&format!("author_tokens:\n  {WORK}: ghp_thework\n"));
    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;

    assert_eq!(
        b.asks_token_of(A, WORK).await,
        Ok(Some("ghp_thework".to_owned())),
    );

    // A uuid A only holds a mirror of: C's Author, and C's to hand over.
    record_author_mirrors(
        &a.pool,
        C,
        &[AuthorMirror {
            id: HOME.to_owned(),
            device: C.to_owned(),
            display_name: String::new(),
            name: "The Third".to_owned(),
            email: "third@example.com".to_owned(),
            holds_token: true,
        }],
    )
    .await
    .unwrap();
    std::fs::write(
        a.dir.path().join("secrets.yaml"),
        format!("author_tokens:\n  {WORK}: ghp_thework\n  {HOME}: ghp_thecopy\n"),
    )
    .unwrap();

    let said = b.asks_token_of(A, HOME).await;
    assert!(
        said.as_ref().is_err_and(|said| said.contains("404")),
        "a mirror there: {said:?}",
    );

    let said = b.asks_token_of(A, "no-such-uuid").await;
    assert!(
        said.as_ref().is_err_and(|said| said.contains("404")),
        "an unknown uuid: {said:?}",
    );

    // And a device A holds no membership for is turned away at the gate.
    let stranger = Verkstead::answering(C).await;
    stranger
        .linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()])
        .await;

    let said = stranger.asks_token_of(A, WORK).await;
    assert!(
        said.as_ref().is_err_and(|said| said.contains("403")),
        "a stranger: {said:?}",
    );
}

impl Verkstead {
    /// One of the list's Authors edited alone, the way the Git section saves a
    /// mirror: the status and what came back.
    async fn edits_author(&self, id: &str, edit: serde_json::Value) -> (StatusCode, String) {
        pressing(
            &self.workbench,
            &format!("/api/ui/authors/{id}"),
            Some(&edit.to_string()),
        )
        .await
    }

    /// And taken away, whatever came of it.
    async fn presses_remove(&self, id: &str) -> (StatusCode, String) {
        pressing(
            &self.workbench,
            &format!("/api/ui/authors/{id}/remove"),
            None,
        )
        .await
    }

    /// A repository registered here as `author`.
    async fn registers(&self, path: &Path, author: &str) {
        repository(path);

        let (status, said) = pressing(
            &self.workbench,
            "/api/ui/repos",
            Some(&serde_json::json!({ "path": path, "author": author }).to_string()),
        )
        .await;

        assert!(status.is_success(), "{status} {said}");
        assert!(said.contains("Added"), "registered: {said}");
    }
}

/// What `config.yaml` in `dir` holds, as text.
fn config_in(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("config.yaml")).unwrap_or_default()
}

/// An edit of a mirror's email made on B lands in A's own `config.yaml`, and
/// B's list says it the moment the press answers, off the row A gave back.
#[tokio::test]
async fn an_edit_of_a_mirror_is_made_at_home() {
    let (a, b, _holding) = linked_up().await;

    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;
    b.authors_saying(|rows| with(rows, WORK).is_some()).await;

    let (status, said) = b
        .edits_author(
            WORK,
            serde_json::json!({
                "display_name": "Work",
                "name": "Ada Lovelace",
                "email": "ada@new.example",
                "github_token": "Keep",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"Saved":{"verified":null}}"#);

    let config = config_in(a.dir.path());
    assert!(config.contains("ada@new.example"), "at home: {config}");
    assert!(
        !config_in(b.dir.path()).contains("ada@new.example"),
        "and nothing of it in B's own files",
    );

    let rows = b.authors().await;
    let row = with(&rows, WORK).expect("still listed");
    assert_eq!(row.email, "ada@new.example", "redrawn off A's row");
    assert!(row.device.is_some(), "still A's");
}

/// A token set on a mirror from B is saved in A's `secrets.yaml`, verified
/// from A, and comes back to B only as the copy.
#[tokio::test]
async fn a_token_set_on_a_mirror_is_saved_and_verified_at_home() {
    let (a, b, _holding) = linked_up().await;

    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;
    b.authors_saying(|rows| with(rows, WORK).is_some()).await;

    let (status, said) = b
        .edits_author(
            WORK,
            serde_json::json!({
                "display_name": "Work",
                "name": "Ada Lovelace",
                "email": "ada@work.example",
                "github_token": { "Set": { "token": "ghp_setfromb" } },
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{said}");

    let said: verkstead_render::AuthorEdited = serde_json::from_str(&said).unwrap();
    assert!(
        matches!(
            said,
            verkstead_render::AuthorEdited::Saved { verified: Some(_) }
        ),
        "with what A's GitHub made of it: {said:?}",
    );

    assert!(
        secrets_in(a.dir.path()).contains(&format!("{WORK}: ghp_setfromb")),
        "saved at home",
    );
    b.secrets_saying(|said| said.contains("ghp_setfromb")).await;

    let rows = b.authors().await;
    assert!(with(&rows, WORK).unwrap().holds_token);
}

/// Removing a mirror from B checks A's Repos alone: refused naming the one at
/// home pointing at it, removed once none does — and B's own Repo that pointed
/// at it is left with an Author that has gone.
#[tokio::test]
async fn a_removal_of_a_mirror_checks_the_homes_repos() {
    let (a, b, _holding) = linked_up().await;

    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;
    a.saves_author(HOME, "Home", "Ada Lovelace", "ada@home.example")
        .await;
    b.authors_saying(|rows| with(rows, WORK).is_some() && with(rows, HOME).is_some())
        .await;

    let repos = tempfile::tempdir().unwrap();
    a.registers(&repos.path().join("at-home"), WORK).await;
    b.registers(&repos.path().join("on-b"), HOME).await;

    let (status, said) = b.presses_remove(WORK).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"InUse":{"repos":["at-home"]}}"#);
    assert!(config_in(a.dir.path()).contains(WORK), "still at home");

    let (status, said) = b.presses_remove(HOME).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, "\"Removed\"", "B's own Repo is not asked about");
    assert!(!config_in(a.dir.path()).contains(HOME), "gone at home");

    let rows = b.authors().await;
    assert!(
        with(&rows, HOME).is_none(),
        "and from the list here at once"
    );

    assert_eq!(
        verkstead_server::settings::Settings::in_data_dir(b.dir.path())
            .config()
            .author_missing(Some(HOME)),
        Some(verkstead_server::settings::Unauthored::Gone),
        "B's Repo refuses its next Start as an Author that has gone",
    );
}

/// With A not answering, an edit and a removal of its Author from B are each
/// refused naming A, and nothing anywhere moves.
#[tokio::test]
async fn a_home_not_answering_refuses_the_press_by_name() {
    let (a, b, holding) = linked_up().await;

    a.saves_author(WORK, "Work", "Ada Lovelace", "ada@work.example")
        .await;
    b.authors_saying(|rows| with(rows, WORK).is_some()).await;

    drop(holding);
    b.linked_to(&a.device, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    let (status, said) = b
        .edits_author(
            WORK,
            serde_json::json!({
                "display_name": "Work",
                "name": "Ada Lovelace",
                "email": "ada@new.example",
                "github_token": "Keep",
            }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{said}");
    assert!(said.contains(A_MACHINE), "naming A: {said}");

    let (status, said) = b.presses_remove(WORK).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{said}");
    assert!(said.contains(A_MACHINE), "naming A: {said}");

    assert!(config_in(a.dir.path()).contains("ada@work.example"));

    let rows = b.authors().await;
    assert_eq!(with(&rows, WORK).unwrap().email, "ada@work.example");
}
