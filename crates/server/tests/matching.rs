//! **Repos across devices**: which of a member's Repos is this repository
//! (ADR-0020, *Repos across devices*).
//!
//! **Two Verksteads, real repositories and a real dial.** `tests/mirroring.rs`
//! is the account list written down over the link; this is the reading beside it
//! that settles a repository. So nothing here hands B a list of A's registry by
//! the back door: every repository in this suite is a directory `git init` really
//! ran in, with the remote it is supposed to have, and B asks A over the Peer
//! Listener behind the Member Gate — the reading a memory sync makes before a
//! launch, and the one a transfer makes again when it has to say where a branch
//! lands.
//!
//! **What every assertion reads is the answer as the asking device gets it**:
//! the Repo row on the machine that answered, or nothing, or a sentence naming a
//! machine that did not take the question. Those last two are the pair worth
//! keeping apart — *nothing over there matched* offers to clone the repository
//! there, and *that machine is asleep* sends the human to the machine.
//!
//! **And it is asked both ways round**, because the question is: the memory sync
//! asks it of the device a Profile's account is on, and a transfer asks it of the
//! device the work is going to.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use sqlx::SqlitePool;
use tower::ServiceExt;
use verkstead_render::{ClonableRepo, Cloned};
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::matching;
use verkstead_server::nudge::Nudges;
use verkstead_server::onboarding::Machine;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::{self, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_server::{Routers, open_database, routers_answering_devices_telling, store};
use verkstead_store::{Linking, record_member};

/// The device whose Repos are asked after.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the device that asks, which is where every assertion is made.
const B: &str = "0011223344556677889900aabbccddee";

/// What A is written down as on B: the name the human gave the machine, which is
/// what a refusal has to name rather than sixteen bytes of hex.
const A_MACHINE: &str = "the-workstation";
const A_OS: &str = "macOS 15.1";

/// How long one address has to answer here, rather than the two seconds a running
/// server gives one.
const PATIENCE: std::time::Duration = std::time::Duration::from_millis(300);

/// One Verkstead: its store, its identity, and its Peer Listener up behind the
/// Member Gate — the cluster handle beside them, because that is what the reading
/// under test is asked with.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    cluster: Devices,
    address: SocketAddr,

    /// What this device's browser reaches, which is where a press made on it —
    /// a clone, relayed to a member or not — is asked.
    workbench: Router,

    /// Held for the length of the test: the identity, the database and every
    /// repository registered on it live in it.
    dir: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead with a store of its own, an identity in it, and both routers
    /// over the one state — the peer half served on a port the machine picked.
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
            Machine::here(),
        );

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
            cluster,
            address,
            workbench,
            dir,
        }
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

    /// A repository on this machine, registered as a Repo — with the `origin`
    /// remote `origin` names, or none at all.
    ///
    /// Really made and really registered: the origin the reading compares is the
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

        // Read back rather than taken from the insert, so that what the reading
        // is given is the row as anything else reading the registry would find
        // it.
        store::registered_repo(&self.pool, repo.id)
            .await
            .unwrap()
            .expect("the Repo that was just registered")
    }

    /// Which of `other`'s Repos is `here`, asked the way the memory sync and a
    /// transfer both ask it.
    async fn which_of(
        &self,
        other: &str,
        here: &store::Repo,
    ) -> Result<Option<verkstead_render::RepoAcross>, verkstead_server::Refusal> {
        matching::across(&self.cluster, other, here).await
    }
}

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a program
/// that is not there, which is what having none *is*.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422)
}

/// A and B, linked both ways.
///
/// Both ways because a link is both ways: B dials A pinned on A's fingerprint,
/// and A admits B because B's certificate is one it holds a membership for — which
/// is also what lets the question be asked in either direction.
async fn linked_up() -> (Verkstead, Verkstead) {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    b.linked_to(&a.device, A_MACHINE, A_OS, vec![a.at()]).await;

    let (machine, os) = this_machine();
    a.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

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

/// One origin checked out at two different paths under two different names is one
/// repository: the row records neither, and the question is asked of git.
#[tokio::test]
async fn one_origin_at_two_paths_under_two_names_matches() {
    let (a, b) = linked_up().await;

    let origin = "https://github.com/you/widgets.git";

    let theirs = a.repo("widgets", Some(origin)).await;
    let ours = b.repo("widgets-clone", Some(origin)).await;

    let matched = b
        .which_of(A, &ours)
        .await
        .expect("A answered")
        .expect("A holds this repository");

    assert_eq!(
        matched.id, theirs.id,
        "the Repo is named by A's own id for it, which is what addresses it there",
    );
    assert_eq!(
        matched.path,
        theirs.path.to_string_lossy(),
        "and by the path on A, which is what a rewrite lands on",
    );
    assert_eq!(matched.name, "widgets", "under the name A gives it");
}

/// And the question goes the other way round, which is what transfer asks it
/// for: the same reading, with the other device named.
#[tokio::test]
async fn the_same_question_is_asked_from_either_end() {
    let (a, b) = linked_up().await;

    let origin = "https://github.com/you/widgets.git";

    let theirs = b.repo("widgets-clone", Some(origin)).await;
    let ours = a.repo("widgets", Some(origin)).await;

    let matched = a
        .which_of(B, &ours)
        .await
        .expect("B answered")
        .expect("B holds this repository");

    assert_eq!(matched.id, theirs.id, "B's own id for its own Repo");
    assert_eq!(matched.name, "widgets-clone");
}

/// Two Repos sharing a name with different origins are two repositories, and two
/// spellings of one URL are one.
#[tokio::test]
async fn a_shared_name_does_not_match_and_two_spellings_do() {
    let (a, b) = linked_up().await;

    a.repo("widgets", Some("https://github.com/someone/widgets.git"))
        .await;

    let ours = b
        .repo("widgets", Some("https://github.com/you/widgets.git"))
        .await;

    assert!(
        b.which_of(A, &ours).await.expect("A answered").is_none(),
        "a name they happen to share is not what makes two checkouts one repository",
    );

    // And the same registry, asked about a repository whose origin is A's own
    // written the other way round: the `scp`-style spelling against `ssh://`,
    // with the `.git` off one end and a slash on the other.
    let theirs = a
        .repo("gadgets", Some("git@github.com:you/gadgets.git"))
        .await;

    let spelled = b
        .repo("gadgets-clone", Some("ssh://git@github.com/you/gadgets/"))
        .await;

    let matched = b
        .which_of(A, &spelled)
        .await
        .expect("A answered")
        .expect("two spellings of one URL are one repository");

    assert_eq!(matched.id, theirs.id, "which is A's gadgets");
}

/// Neither end has an origin and the names agree: a match, that being all there
/// is to go on. One has an origin and the other has not: no match.
#[tokio::test]
async fn with_no_origin_the_names_decide_and_one_origin_is_enough_to_refuse() {
    let (a, b) = linked_up().await;

    let theirs = a.repo("notes", None).await;
    let ours = b.repo("notes", None).await;

    let matched = b
        .which_of(A, &ours)
        .await
        .expect("A answered")
        .expect("neither has an origin and the names agree");

    assert_eq!(matched.id, theirs.id, "A's notes");

    // And a repository of the same name that A has an origin for: two
    // repositories until something says otherwise, and a shared name is not that
    // something.
    a.repo("sketches", Some("https://github.com/you/sketches.git"))
        .await;

    let ours = b.repo("sketches", None).await;

    assert!(
        b.which_of(A, &ours).await.expect("A answered").is_none(),
        "a repository with an origin is not the one without it that shares its name",
    );
}

/// Asked of a device that is not answering, it is **refused naming the device**
/// rather than answered that nothing matched.
///
/// Those are two different things to say, and a preflight that confused them
/// would offer to clone the repository onto a machine that already has it.
#[tokio::test]
async fn an_unreachable_device_is_refused_by_name() {
    let b = Verkstead::answering(B).await;

    // A device that is a member and is not there: the identity is real, so the
    // membership row is a real one, and nothing is listening at the address.
    let away = tempfile::tempdir().unwrap();
    let asleep = Device::stated(away.path(), A).unwrap();

    b.linked_to(&asleep, A_MACHINE, A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    let ours = b
        .repo("widgets", Some("https://github.com/you/widgets.git"))
        .await;

    let refusal = b
        .which_of(A, &ours)
        .await
        .expect_err("a machine that did not answer is not an answer of no match");

    assert_eq!(
        refusal.status,
        StatusCode::BAD_GATEWAY,
        "the far end's trouble rather than this one's: {}",
        refusal.saying,
    );
    assert!(
        refusal.saying.contains(A_MACHINE),
        "the machine is named by the name the human gave it: {}",
        refusal.saying,
    );
}

/// And a device that is no member of this cluster at all is refused too, rather
/// than answered no match: nothing was asked of anybody.
#[tokio::test]
async fn a_device_that_is_no_member_is_refused_as_well() {
    let b = Verkstead::answering(B).await;

    let ours = b.repo("widgets", None).await;

    let refusal = b
        .which_of(A, &ours)
        .await
        .expect_err("a device this one holds no membership for was never asked");

    assert_eq!(refusal.status, StatusCode::BAD_GATEWAY);
    assert!(
        refusal.saying.contains("member"),
        "it says the device is not one of this device's members: {}",
        refusal.saying,
    );
}

/// A Repo the human has taken off the registry is not on the answer: a
/// repository this device has been told to stop offering is not one to land
/// somebody else's work in.
#[tokio::test]
async fn an_unregistered_repo_is_not_matched() {
    let (a, b) = linked_up().await;

    let origin = "https://github.com/you/widgets.git";

    let theirs = a.repo("widgets", Some(origin)).await;
    let ours = b.repo("widgets-clone", Some(origin)).await;

    assert!(
        b.which_of(A, &ours).await.expect("A answered").is_some(),
        "it matches while the Repo is registered",
    );

    store::unregister_repo(&a.pool, theirs.id).await.unwrap();

    assert!(
        b.which_of(A, &ours).await.expect("A answered").is_none(),
        "and stops the moment A stops offering it",
    );
}

/// Asked with the origins beside the matches, as the preflight asks it, each of
/// this device's Repos comes back with the origin git reads off it here — the
/// one a finding with no match is cloned from — and one with none, with none.
#[tokio::test]
async fn each_repo_comes_back_with_the_origin_it_was_matched_on() {
    let (a, b) = linked_up().await;

    let held = "https://github.com/you/widgets.git";
    let lacked = "https://github.com/you/askance.git";

    a.repo("widgets-clone", Some(held)).await;

    let ours = [
        b.repo("widgets", Some(held)).await,
        b.repo("askance", Some(lacked)).await,
        b.repo("scratch", None).await,
    ];

    let read = matching::each_with_origin(&b.cluster, A, &ours)
        .await
        .expect("A answered");

    let read: Vec<(Option<&str>, bool)> = read
        .iter()
        .map(|(origin, matched)| (origin.as_deref(), matched.is_some()))
        .collect();

    assert_eq!(
        read,
        vec![(Some(held), true), (Some(lacked), false), (None, false)],
        "in the order asked, each with its own origin and whether A holds it",
    );
}

/// A third device, for the readings that ask every member at once.
const C: &str = "c0ffee00c0ffee00c0ffee00c0ffee00";

impl Verkstead {
    /// What the cluster holds that this device could clone, asked the way the
    /// Repo dropdown asks it: against every Repo this device has registered.
    async fn clonable(&self) -> Vec<ClonableRepo> {
        let here = store::registered_repos(&self.pool).await.unwrap();

        matching::clonable(&self.cluster, &here)
            .await
            .expect("only this device's own trouble refuses the reading")
    }
}

/// The device lacking a repository lists it as clonable, named off its origin;
/// the device holding it does not.
#[tokio::test]
async fn what_one_member_holds_and_another_lacks_is_clonable_there() {
    let (a, b) = linked_up().await;

    // A host no `insteadOf` rewrites, so the spelling read back is the one
    // written: a machine set up to fetch GitHub over https says so for its SSH
    // URLs too.
    a.repo("widgets-work", Some("git@git.example.com:you/widgets.git"))
        .await;
    b.repo("gadgets", Some("https://github.com/you/gadgets.git"))
        .await;

    // And one both hold, under two names and two spellings: neither lists it.
    a.repo("shared", Some("https://github.com/you/shared.git"))
        .await;
    b.repo("shared-clone", Some("https://github.com/you/shared/"))
        .await;

    assert_eq!(
        b.clonable().await,
        vec![ClonableRepo {
            name: "widgets".to_owned(),
            place: "git.example.com/you/widgets".to_owned(),
            origin: "git@git.example.com:you/widgets.git".to_owned(),
            author: None,
        }],
        "B lacks A's widgets, named off the origin rather than off A's directory",
    );

    assert_eq!(
        a.clonable().await,
        vec![ClonableRepo {
            name: "gadgets".to_owned(),
            place: "github.com/you/gadgets".to_owned(),
            origin: "https://github.com/you/gadgets.git".to_owned(),
            author: None,
        }],
        "and A lacks B's gadgets, but not the widgets it holds itself",
    );
}

/// A Repo with no origin is never clonable, a shared name hides nothing, and one
/// origin held by two members in two spellings is one row.
#[tokio::test]
async fn one_origin_is_one_row_and_no_origin_is_none() {
    let (a, b) = linked_up().await;
    let c = Verkstead::answering(C).await;

    b.linked_to(&c.device, "the-laptop", A_OS, vec![c.at()])
        .await;
    let (machine, os) = this_machine();
    c.linked_to(&b.device, &machine, &os, vec![b.at()]).await;

    a.repo("notes", None).await;
    a.repo("widgets", Some("https://github.com/you/widgets.git"))
        .await;
    c.repo("widgets-copy", Some("https://github.com/you/widgets/"))
        .await;

    // B's own `gadgets` has no origin, so it is not A's — a name is no reason to
    // hide A's from the list.
    a.repo("gadgets", Some("https://github.com/you/gadgets.git"))
        .await;
    b.repo("gadgets", None).await;

    let listed = b.clonable().await;
    let names: Vec<&str> = listed.iter().map(|row| row.name.as_str()).collect();

    assert_eq!(
        names,
        vec!["gadgets", "widgets"],
        "one row for the origin two members hold, and none for a Repo with no origin: \
         {listed:?}",
    );
}

/// A member that does not answer — nothing at its address, or something there
/// that takes the connection and says nothing — adds no rows, fails nothing, and
/// holds the reading up no longer than its bound.
#[tokio::test]
async fn a_member_that_does_not_answer_adds_nothing_and_holds_nothing_up() {
    let (a, b) = linked_up().await;

    a.repo("widgets", Some("https://github.com/you/widgets.git"))
        .await;

    // A member that is not there at all.
    let away = tempfile::tempdir().unwrap();
    let asleep = Device::stated(away.path(), C).unwrap();
    b.linked_to(&asleep, "the-laptop", A_OS, vec!["127.0.0.1:1".to_owned()])
        .await;

    // And one that takes the connection and never says a word.
    let mute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mute_at = mute.local_addr().unwrap().to_string();
    std::thread::spawn(move || {
        let held: Vec<_> = mute.incoming().collect();
        drop(held);
    });

    let quiet = tempfile::tempdir().unwrap();
    let silent = Device::stated(quiet.path(), "dd00dd00dd00dd00dd00dd00dd00dd00").unwrap();
    b.linked_to(&silent, "the-nas", A_OS, vec![mute_at]).await;

    let started = std::time::Instant::now();
    let listed = b.clonable().await;

    assert_eq!(
        listed
            .iter()
            .map(|row| row.name.as_str())
            .collect::<Vec<_>>(),
        vec!["widgets"],
        "the member that answered is read, and the two that did not add nothing",
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "and neither held the reading past its bound: {:?}",
        started.elapsed(),
    );
}

impl Verkstead {
    /// Clone `origin` under `parent` on `device`, pressed on this device's
    /// workbench the way the Repo dropdown presses it: at its own path where
    /// `device` is this one, and over the Relay where it is a member.
    async fn clone_on(&self, device: &str, origin: &str, parent: &Path) -> Cloned {
        let path = match device == self.device.id() {
            true => "/api/ui/repos/clone".to_owned(),
            false => format!("/api/ui/members/{device}/repos/clone"),
        };

        let body = serde_json::to_vec(&serde_json::json!({
            "origin": origin,
            "parent": parent.to_string_lossy(),
        }))
        .unwrap();

        let answered = self
            .workbench
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&path)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::CONTENT_LENGTH, body.len())
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = answered.status();
        let bytes = answered.into_body().collect().await.unwrap().to_bytes();
        let said = String::from_utf8_lossy(&bytes);

        assert_eq!(status, StatusCode::OK, "POST {path}: {said}");
        serde_json::from_str(&said).unwrap_or_else(|why| panic!("{path} answered {said}: {why}"))
    }

    /// Every Repo registered here.
    async fn registered(&self) -> Vec<store::Repo> {
        store::registered_repos(&self.pool).await.unwrap()
    }
}

/// A repository on A whose origin is a real one a clone can be made from: a bare
/// copy of it, named `name.git` in a directory of its own, so the origin's last
/// path segment is `name` whatever A calls its checkout.
async fn held_with_a_real_origin(a: &Verkstead, checkout: &str, name: &str) -> String {
    let origins = a.dir.path().join("origins");
    std::fs::create_dir_all(&origins).unwrap();

    let source = repository(a.dir.path().join("sources").join(name));
    let bare = origins.join(format!("{name}.git"));
    git(
        &origins,
        &[
            "clone",
            "--bare",
            "--quiet",
            &source.to_string_lossy(),
            &bare.to_string_lossy(),
        ],
    );

    let origin = bare.to_string_lossy().into_owned();
    a.repo(checkout, Some(&origin)).await;

    origin
}

/// Pressing a clonable row on a member: B reads what it lacks, A's browser
/// clones it onto B over the Relay, and the Repo B registers is named off the
/// origin. Asking again answers the same Repo, and nothing registers twice.
#[tokio::test]
async fn a_clonable_repo_clones_onto_the_member_lacking_it() {
    let (a, b) = linked_up().await;

    let origin = held_with_a_real_origin(&a, "widgets-work", "widgets").await;

    let row = b
        .clonable()
        .await
        .into_iter()
        .find(|row| row.origin == origin)
        .expect("B lists A's widgets as clonable");

    let parent = b.dir.path().join("clones");
    std::fs::create_dir_all(&parent).unwrap();

    let cloned = a.clone_on(B, &row.origin, &parent).await;
    let Cloned::Cloned(repo) = cloned else {
        panic!("the clone lands on B: {cloned:?}");
    };

    assert_eq!(
        repo.name, "widgets",
        "named off the origin, not off A's checkout"
    );
    assert!(
        Path::new(&repo.path).join("README.md").is_file(),
        "and it is the repository, cloned: {}",
        repo.path,
    );
    assert_eq!(repo.default_branch, "main");
    assert_eq!(
        b.registered()
            .await
            .iter()
            .map(|held| held.id)
            .collect::<Vec<_>>(),
        vec![repo.id],
        "registered on B",
    );
    assert!(
        a.registered()
            .await
            .iter()
            .all(|held| held.name != "widgets")
    );

    assert!(
        b.clonable().await.iter().all(|row| row.origin != origin),
        "and B lacks it no longer",
    );

    let again = a.clone_on(B, &row.origin, &parent).await;

    assert_eq!(
        again,
        Cloned::AlreadyRegistered(repo.clone()),
        "the same path again is the same Repo",
    );
    assert_eq!(
        b.registered().await.len(),
        1,
        "with nothing registered twice"
    );
}

/// A clone git refuses is git's own words, registers nothing, and leaves no
/// directory it made — and a directory somebody else put there is refused and
/// left exactly as it was.
#[tokio::test]
async fn a_clone_git_refuses_leaves_nothing_behind() {
    let (a, b) = linked_up().await;

    let parent = b.dir.path().join("clones");
    std::fs::create_dir_all(&parent).unwrap();

    let nowhere = a.dir.path().join("origins").join("gone.git");

    let Cloned::Refused(why) = a.clone_on(B, &nowhere.to_string_lossy(), &parent).await else {
        panic!("an origin with nothing at it is refused");
    };

    assert!(why.starts_with("fatal:"), "in git's own words: {why}");
    assert!(
        !parent.join("gone").exists(),
        "and no directory is left behind"
    );
    assert!(b.registered().await.is_empty(), "and nothing is registered");

    // A directory of that name that nobody registered: git's to refuse, and
    // nobody's to take away.
    let origin = held_with_a_real_origin(&a, "gadgets-work", "gadgets").await;
    let theirs = parent.join("gadgets");
    std::fs::create_dir_all(&theirs).unwrap();
    std::fs::write(theirs.join("notes.txt"), "mine\n").unwrap();

    let Cloned::Refused(why) = b.clone_on(B, &origin, &parent).await else {
        panic!("a directory that is there is refused");
    };

    assert!(why.contains("already exists"), "in git's own words: {why}");
    assert_eq!(
        std::fs::read_to_string(theirs.join("notes.txt")).unwrap(),
        "mine\n",
        "and what was there is left alone",
    );
    assert!(b.registered().await.is_empty(), "and nothing is registered");

    // And a parent that is not there is the create's own sentence.
    assert_eq!(
        b.clone_on(B, &origin, &parent.join("not-here")).await,
        Cloned::ParentMissing,
    );
    assert_eq!(
        b.clone_on(B, &origin, Path::new("relative/clones")).await,
        Cloned::ParentMissing,
    );
}
