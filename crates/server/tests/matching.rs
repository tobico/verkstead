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
//! keeping apart — *nothing over there matched* sends the human to **Open repo**,
//! and *that machine is asleep* sends them to the machine.
//!
//! **And it is asked both ways round**, because the question is: the memory sync
//! asks it of the device a Profile's account is on, and a transfer asks it of the
//! device the work is going to.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use axum::http::StatusCode;
use sqlx::SqlitePool;
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

        let Routers { over_the_link, .. } = routers_answering_devices_telling(
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
/// would send somebody to **Open repo** on a machine that already has the
/// repository.
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
