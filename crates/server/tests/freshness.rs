//! The news coming back over the link: one Nudge stream held to each member,
//! and everything down it announced here under the device it came from
//! (ADR-0020, *The opened device relays*).
//!
//! **Two Verksteads, a real dial and a real stream.** `tests/relaying.rs` is the
//! hop a browser makes through this device; this is what makes a page drawn out
//! of that hop stay fresh. So nothing here stubs a stream: A holds B's own
//! `/api/ui/nudges` over the Peer Listener, and what is read is A's own stream —
//! the one an open page listens on — as the page would read it.
//!
//! **A Set answered on B is the criterion**, and it is answered the way the
//! human answers one: a press on A's workbench, under the member's prefix,
//! relayed to B. Nothing in this file announces a Nudge by hand — what goes down
//! A's stream got there by B's store settling and B's stream saying so.
//!
//! **And the frames are read as JSON rather than as a type**, which is the other
//! half of the same criterion: a local Nudge has to go on meaning exactly what it
//! means now, and what says so is the bytes. So the assertions are literal
//! objects — a member's carries its Device Id and this device's own carries no
//! such key at all.

use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use serde_json::json;
use sqlx::SqlitePool;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use verkstead_render::Submitted;
use verkstead_schema::{Answer, QuestionSet, Response};
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::{
    open_database, router_answering_devices_telling, router_over_the_link_telling, store,
};
use verkstead_store::{Linking, record_member};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The device the browser is on, which holds the streams.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The device whose work is reached through it.
const B: &str = "0011223344556677889900aabbccddee";

/// The branch each device's one Conversation is on.
const BRANCH: &str = "solid-viewer";

/// And the Conversation itself, which is the first one in a store with nothing
/// else in it — the same number on both devices, ids being each device's own.
const THERE: i64 = 1;

/// How long one address has to answer here, rather than the two seconds a
/// running server gives one.
const PATIENCE: Duration = Duration::from_millis(300);

/// How long a test will wait for a Nudge it expects. Generous, because it is
/// only ever paid when the assertion is about to fail.
const WAITING: Duration = Duration::from_secs(10);

/// And how long for one that has to be waited out: a stream taken up again after
/// the link under it was cut, which the holder does five seconds after the last
/// one ended.
const TAKEN_UP_AGAIN: Duration = Duration::from_secs(20);

/// How long to keep listening after everything expected has arrived, to catch a
/// Nudge that should not have been sent at all.
const SETTLING: Duration = Duration::from_millis(500);

/// One Verkstead: its store, its identity, the Nudge channel its own pages and
/// its members both read, and where its Peer Listener landed.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    members: Members,
    reading: Reading,

    /// What this device announces on — the one handle behind both of its
    /// routers, as it is in a running server: a member reads it over the link,
    /// and this device's own pages read it at `/api/ui/nudges`.
    nudges: Nudges,

    /// Where the other device dials it, on a port the machine picked.
    address: SocketAddr,

    /// Held for the length of the test: the identity and the database live in
    /// it.
    _dir: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead with a store of its own, an identity in it, and its Peer
    /// Listener up — serving the viewer's namespace behind the Member Gate, over
    /// the Nudge channel this holds.
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

        tokio::spawn(listener.serving(peer::router(
            device.clone(),
            reading.clone(),
            members.clone(),
            Joins::none(),
            nudges.clone(),
            router_over_the_link_telling(pool.clone(), dir.path().to_owned(), nudges.clone()),
        )));

        Verkstead {
            device,
            pool,
            members,
            reading,
            nudges,
            address,
            _dir: dir,
        }
    }

    /// This device's cluster, with every dial given [`PATIENCE`].
    fn cluster(&self) -> Devices {
        Devices::of(
            self.device.clone(),
            self.reading.clone(),
            self.members.clone(),
            Joins::none(),
        )
        .waiting(PATIENCE)
    }

    /// The workbench its own browser talks to, over the same Nudge channel the
    /// streams announce on.
    fn workbench(&self) -> Router {
        router_answering_devices_telling(self.pool.clone(), self.cluster(), self.nudges.clone())
    }

    /// Hold a Nudge stream to every member of this device's cluster, which is
    /// what a running server spawns at the start.
    ///
    /// The handle is kept by the caller so the streams are let go of when the
    /// test ends rather than left dialling a listener that has gone.
    fn holding(&self) -> Holding {
        let cluster = self.cluster();
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
    async fn linked_to(&self, other: &Device, addresses: Vec<String>) {
        record_member(
            &self.pool,
            &Linking {
                device: other.id().to_owned(),
                name: "somewhere-else".to_owned(),
                os: "Linux".to_owned(),
                addresses,
                fingerprint: other.fingerprint().to_owned(),
            },
        )
        .await
        .unwrap();
    }

    /// One Repo and one Conversation in its store, so that a Set has a Timeline
    /// to land on.
    async fn holding_work(&self) {
        let repo =
            store::register_repo(&self.pool, Path::new("/srv/verkstead"), "verkstead", "main")
                .await
                .unwrap()
                .expect("nothing is registered at that path yet");

        let conversation = store::start_conversation(&self.pool, repo.id, BRANCH, THIS_DEVICE)
            .await
            .unwrap()
            .expect("the Repo was just registered");

        assert_eq!(conversation, THERE);
    }

    /// A Set waiting on the human, put straight into its store.
    async fn asking(&self, title: &str) -> i64 {
        store::ask(&self.pool, THERE, &bare(title), store::Ask::Blocking)
            .await
            .unwrap()
            .expect("the Conversation is there to ask from")
            .id
    }
}

/// The streams one device is holding, given back when the test drops it.
struct Holding(JoinHandle<()>);

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

/// A and B, linked both ways, with work in B's store — and A's row for B at
/// whatever `reaching` says instead of B's own address.
///
/// Both ways because a link is both ways: A dials B pinned on B's fingerprint,
/// and B admits A because A's certificate is one it holds a membership for.
async fn linked(reaching: impl Fn(&Verkstead) -> String) -> (Verkstead, Verkstead) {
    let a = Verkstead::answering(A).await;
    let b = Verkstead::answering(B).await;

    b.holding_work().await;

    a.linked_to(&b.device, vec![reaching(&b)]).await;
    b.linked_to(&a.device, vec![a.at()]).await;

    (a, b)
}

/// The same, with A reaching B at B's own address, which is every test here but
/// the one about a link that drops.
async fn linked_up() -> (Verkstead, Verkstead) {
    linked(Verkstead::at).await
}

/// A link between two devices as something that can be **cut**: a socket on the
/// loopback carrying bytes to the far end and back, which lets go of every
/// connection it is holding when it is told to.
///
/// **Which is what a dropped link is**, and the one thing a test cannot do by
/// stopping the far end: axum serves each connection in a task of its own, so a
/// listener taken away leaves the stream A is holding open. The bytes crossing
/// here are the handshake and the stream inside it, untouched — the certificate
/// A pins is still B's, because this carries B's own TLS rather than standing in
/// for it.
struct Link {
    /// Where A dials, which is what A's membership row for B says.
    address: SocketAddr,

    cutting: broadcast::Sender<()>,
}

impl Link {
    /// A link to `far`, open until it is cut.
    async fn to(far: SocketAddr) -> Link {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (cutting, _) = broadcast::channel(1);
        let cut = cutting.clone();

        tokio::spawn(async move {
            while let Ok((mut near, _)) = listener.accept().await {
                let mut cut = cut.subscribe();

                tokio::spawn(async move {
                    let Ok(mut across) = TcpStream::connect(far).await else {
                        return;
                    };

                    tokio::select! {
                        _ = tokio::io::copy_bidirectional(&mut near, &mut across) => {}
                        // Both sockets are dropped with this task, which is what
                        // the far end and the near end each see as the link
                        // going down.
                        _ = cut.recv() => {}
                    }
                });
            }
        });

        Link { address, cutting }
    }

    fn at(&self) -> String {
        format!("127.0.0.1:{}", self.address.port())
    }

    fn cut(&self) {
        self.cutting
            .send(())
            .expect("the link holds its own receiver");
    }
}

/// An open page, listening on this device's own Nudge stream.
struct Listening {
    body: Body,
    buffered: String,
}

impl Listening {
    async fn open(app: &Router) -> Listening {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/ui/nudges")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|kind| kind.to_str().ok()),
            Some("text/event-stream"),
        );

        Listening {
            body: response.into_body(),
            buffered: String::new(),
        }
    }

    /// The next whole frame off the stream, whatever kind it is.
    async fn frame(&mut self) -> String {
        loop {
            if let Some(end) = self.buffered.find("\n\n") {
                return self.buffered.drain(..end + 2).collect();
            }

            let chunk = self
                .body
                .frame()
                .await
                .expect("the stream ended")
                .unwrap()
                .into_data()
                .expect("the stream carries data frames");

            self.buffered.push_str(std::str::from_utf8(&chunk).unwrap());
        }
    }

    /// The next Nudge as the page reads it — the JSON of the `data` line, past
    /// the keep-alives that are the stream's other traffic.
    async fn nudge(&mut self) -> serde_json::Value {
        self.waiting(WAITING).await
    }

    /// The same, given as long as a stream being taken up again takes.
    async fn nudge_within(&mut self, patience: Duration) -> serde_json::Value {
        self.waiting(patience).await
    }

    async fn waiting(&mut self, patience: Duration) -> serde_json::Value {
        let waited_for = tokio::time::timeout(patience, async {
            loop {
                let frame = self.frame().await;

                if frame.starts_with("event: nudge") {
                    return said(&frame);
                }
            }
        });

        waited_for.await.expect("waited for a Nudge in vain")
    }

    /// Insist that nothing more is coming.
    async fn nothing_more(&mut self) {
        let arrived = tokio::time::timeout(SETTLING, async {
            loop {
                let frame = self.frame().await;

                if frame.starts_with("event: nudge") {
                    return frame;
                }
            }
        })
        .await;

        assert!(arrived.is_err(), "an unwanted Nudge arrived: {arrived:?}");
    }
}

/// What one frame said: the JSON of its `data` line, read as the page reads it.
fn said(frame: &str) -> serde_json::Value {
    let data = frame
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap_or_else(|| panic!("a Nudge frame carries a data line, not {frame:?}"));

    serde_json::from_str(data)
        .unwrap_or_else(|error| panic!("a Nudge should be readable as one: {data:?} — {error}"))
}

/// Where a local call stands when it is for `device` instead: the prefix takes
/// the place of `/api/ui`, and everything under it is untouched.
fn through(device: &str, path: &str) -> String {
    let leaf = path
        .strip_prefix("/api/ui")
        .expect("a relayed call is a call into the viewer's own namespace");

    format!("/api/ui/members/{device}{leaf}")
}

/// A press with a JSON body, made the way the browser makes one.
async fn post<T: DeserializeOwned>(app: &Router, path: &str, body: &serde_json::Value) -> T {
    let body = serde_json::to_vec(body).unwrap();

    let answered = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::CONTENT_LENGTH, body.len())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = answered.status();
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();
    let said = String::from_utf8_lossy(&bytes).into_owned();

    assert_eq!(status, StatusCode::OK, "POST {path}: {said}");
    serde_json::from_str(&said).unwrap_or_else(|why| panic!("POST {path} answered {said}: {why}"))
}

/// A Set with nothing but a title, put straight into a store.
fn bare(title: &str) -> QuestionSet {
    QuestionSet {
        title: title.to_owned(),
        preface: None,
        questions: Vec::new(),
        postscript: None,
        proposal: None,
        project: Some("verkstead".to_owned()),
        branch: Some(BRANCH.to_owned()),
        diff: None,
        diffs: Vec::new(),
    }
}

/// And a Response to one, as the viewer builds it.
fn answered(comment: &str) -> serde_json::Value {
    serde_json::to_value(Response {
        answers: Vec::<Answer>::new(),
        comment: Some(comment.to_owned()),
        direction: None,
        nothing_else: false,
    })
    .unwrap()
}

/// Answer `set` on B, the way the human answers one from A's browser: a press on
/// A's workbench under B's Device Id, relayed to B.
async fn answer_on_the_member(workbench: &Router, set: i64) {
    let outcome: Submitted = post(
        workbench,
        &through(B, &format!("/api/ui/sets/{set}/response")),
        &answered("answered from the other device"),
    )
    .await;

    assert_eq!(outcome, Submitted::Accepted, "the press is B's own answer");
}

/// A Set answered on B refreshes A's open page on that Conversation: the news
/// comes down the stream A holds to B, and out on the stream A's own page is
/// listening to — with B's Device Id on it, which is what makes the page read
/// back B's Conversation rather than its own.
#[tokio::test]
async fn a_set_answered_on_a_member_lands_on_this_devices_stream_as_that_members() {
    let (a, b) = linked_up().await;
    let set = b.asking("what shall we call it?").await;

    let workbench = a.workbench();
    let mut page = Listening::open(&workbench).await;
    let _holding = a.holding();

    // The stream being taken up at all, which is the first thing said under a
    // member: a page that was drawn while B was not answering reads it back now.
    assert_eq!(
        page.nudge().await,
        json!({ "kind": "everything", "device": B }),
    );

    answer_on_the_member(&workbench, set).await;

    assert_eq!(
        page.nudge().await,
        json!({ "kind": "set", "conversation": THERE, "device": B }),
        "the Set settled on B, and A's page is told it was B's that moved",
    );
}

/// And nothing of A's own is said to have been a member's: a Set answered on this
/// device carries no device at all, which is the frame it has always been.
#[tokio::test]
async fn this_devices_own_news_carries_no_device() {
    let a = Verkstead::answering(A).await;
    a.holding_work().await;
    let set = a.asking("what shall we call it?").await;

    let workbench = a.workbench();
    let mut page = Listening::open(&workbench).await;

    let outcome: Submitted = post(
        &workbench,
        &format!("/api/ui/sets/{set}/response"),
        &answered("answered here"),
    )
    .await;
    assert_eq!(outcome, Submitted::Accepted);

    assert_eq!(
        page.nudge().await,
        json!({ "kind": "set", "conversation": THERE }),
        "a local Nudge is the two fields it always was, and carries no Device Id",
    );
}

/// A stream whose link drops is taken up again, and coming back announces enough
/// under that device for an open page to read back what it missed.
///
/// The link is cut rather than the far end stopped, for the reason [`Link`]
/// gives. What A sees is what it sees when a laptop's lid shuts: bytes stop, and
/// the stream it was holding ends.
#[tokio::test]
async fn a_stream_whose_link_drops_is_taken_up_again() {
    let b = Verkstead::answering(B).await;
    let link = Link::to(b.address).await;

    let a = Verkstead::answering(A).await;
    b.holding_work().await;
    a.linked_to(&b.device, vec![link.at()]).await;
    b.linked_to(&a.device, vec![a.at()]).await;

    let workbench = a.workbench();
    let mut page = Listening::open(&workbench).await;
    let _holding = a.holding();

    assert_eq!(
        page.nudge().await,
        json!({ "kind": "everything", "device": B }),
        "the stream is taken up, and says so under B",
    );

    link.cut();

    assert_eq!(
        page.nudge_within(TAKEN_UP_AGAIN).await,
        json!({ "kind": "everything", "device": B }),
        "and taken up again once the link is back, saying everything of B's moved: \
         what it missed while it was down is unknowable, so the page reads back \
         what it is showing of that device",
    );

    // And what it is now is a stream like any other: news off it arrives as B's.
    let set = b.asking("and after that?").await;
    answer_on_the_member(&workbench, set).await;

    assert_eq!(
        page.nudge().await,
        json!({ "kind": "set", "conversation": THERE, "device": B }),
    );
}

/// A member's news does not come back around as this device's own.
///
/// Both devices hold a stream to the other, which is what a cluster is: every
/// device holds the whole membership. So what B says has to be B's on A's stream
/// and stop there — a device that passed on what a *third* device told it would
/// have A re-announcing its own news as B's, B re-announcing that as A's, and a
/// Set answered once going round the cluster for ever.
#[tokio::test]
async fn a_members_news_does_not_go_round_the_cluster() {
    let (a, b) = linked_up().await;
    let set = b.asking("what shall we call it?").await;

    let workbench = a.workbench();
    let mut page = Listening::open(&workbench).await;
    let _here = a.holding();
    let _there = b.holding();

    assert_eq!(
        page.nudge().await,
        json!({ "kind": "everything", "device": B }),
    );

    answer_on_the_member(&workbench, set).await;

    assert_eq!(
        page.nudge().await,
        json!({ "kind": "set", "conversation": THERE, "device": B }),
    );

    // One Set answered once, said once. Nothing comes back from B about what A
    // just announced, and nothing goes round again.
    page.nothing_more().await;
}
