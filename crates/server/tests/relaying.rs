//! The hop itself: a call made against one device, put to one of its members
//! over the Peer Listener, and answered back untouched (ADR-0020, *The opened
//! device relays*).
//!
//! **Two Verksteads, and a real dial between them.** `tests/relayed.rs` is the
//! far end of this — the viewer's namespace answered behind the Member Gate —
//! and this is the near end: A's workbench asked the way a browser asks it,
//! reaching B's store through a socket, a handshake and a certificate. Nothing
//! here stubs the hop, because the hop is the whole of what is being asked
//! about.
//!
//! **A's browser holds A's cookie and nothing else**, which is the criterion
//! read backwards: the router these calls go to is stood behind an open gate,
//! and what gets *through to B* is A's certificate. No key of B's is anywhere
//! in this file, and there is nowhere one could be put.
//!
//! **The two devices really do record each other.** B admits A because A is in
//! B's membership, and A dials B because B is in A's — the same pair of rows a
//! join writes, written by hand for the reason `tests/relayed.rs` writes its
//! one by hand: what a membership comes *from* is `tests/joining.rs`'s.

use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, Bytes};
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;
use tokio_stream::Stream;
use tower::ServiceExt;
use verkstead_render::{Attached, ConversationView, Submitted};
use verkstead_schema::{Answer, QuestionSet, Response};
use verkstead_server::attachments::MAX_BYTES;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, Devices};
use verkstead_server::nudge::Nudges;
use verkstead_server::peer::joining::Joins;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::{
    open_database, router_answering_devices_telling, router_keeping, router_over_the_link, store,
};
use verkstead_store::{Linking, record_member};

/// The device the browser is on, which relays.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The device its Conversation lives on, which is reached through the hop.
const B: &str = "0011223344556677889900aabbccddee";

/// A third that A is linked to and that is not switched on, which is what a row
/// nothing answers at is.
const GONE: &str = "99887766554433221100aabbccddeeff";

/// And one A is linked to at all, which is every other device there is.
const A_STRANGER: &str = "ffeeddccbbaa00998877665544332211";

/// The branch B's one Conversation is on, so that what comes back through the
/// hop is assertable as something rather than merely as a 200.
const BRANCH: &str = "solid-viewer";

/// B's Conversation, which is the first one in a store with nothing else in it.
const THERE: i64 = 1;

/// How long one address has to answer here, rather than the two seconds a
/// running server gives one.
///
/// Spent by the test about a member that is not there: what that is asking is
/// what the hop says when nobody is home, and a test that waited out the real
/// deadline would be spending its time on the clock rather than on the
/// question.
const PATIENCE: Duration = Duration::from_millis(300);

/// One Verkstead: its store, its identity, where it keeps what it makes, and
/// where its Peer Listener landed.
struct Verkstead {
    device: Device,
    pool: SqlitePool,
    members: Members,
    reading: Reading,

    /// Where the other device dials it. The operating system's port rather than
    /// 8423, for the reason every other suite here takes one: two of these
    /// running at once must not fight each other.
    address: SocketAddr,

    /// Held for the length of the test: the identity, the database and the
    /// attachments all live in it.
    dir: tempfile::TempDir,
}

impl Verkstead {
    /// A Verkstead with a store of its own, an identity in it, and its Peer
    /// Listener up on a port the machine picked — serving `over_the_link`
    /// behind the Member Gate.
    async fn answering(id: &str, over_the_link: bool) -> Verkstead {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        let device = Device::stated(dir.path(), id).unwrap();
        let members = Members::recorded(pool.clone());

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
            Nudges::new(),
            // The far end of the hop, or nothing at all: A is the device the
            // browser opened and serves its members nothing in this file.
            match over_the_link {
                true => router_over_the_link(pool.clone(), dir.path().to_owned()),
                false => Router::new(),
            },
        )));

        Verkstead {
            device,
            pool,
            members,
            reading,
            address,
            dir,
        }
    }

    /// The workbench its own browser talks to, with every dial given
    /// [`PATIENCE`].
    ///
    /// Built afresh on each call and over the same store, the way the other
    /// device suites here build one.
    fn workbench(&self) -> Router {
        router_answering_devices_telling(
            self.pool.clone(),
            Devices::of(
                self.device.clone(),
                self.reading.clone(),
                self.members.clone(),
                Joins::none(),
            )
            .waiting(PATIENCE),
            Nudges::new(),
        )
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
}

/// A machine with no Tailscale on it: `verkstead-no-such-tailscale` is a
/// program that is not there, which is what having none *is*. Never run, the
/// readings here saying their addresses outright.
fn no_tailscale() -> Tailscale {
    Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422)
}

/// A port on the loopback that nothing is on at all: one taken and given
/// straight back, which is what an address a device has left looks like.
fn nothing_there() -> String {
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = taken.local_addr().unwrap().port();

    drop(taken);

    format!("127.0.0.1:{port}")
}

/// A device standing on its own two files, for the ones nothing stands a
/// listener up for.
fn a_device_called(id: &str) -> (Device, tempfile::TempDir) {
    let elsewhere = tempfile::tempdir().unwrap();
    let device = Device::stated(elsewhere.path(), id).unwrap();

    (device, elsewhere)
}

/// A and B, linked both ways, with one Repo and one Conversation in B's store.
///
/// Both ways because a link is both ways: A dials B pinned on B's fingerprint,
/// and B admits A because A's certificate is one it holds a membership for.
async fn linked() -> (Verkstead, Verkstead) {
    let a = Verkstead::answering(A, false).await;
    let b = Verkstead::answering(B, true).await;

    let repo = store::register_repo(&b.pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let conversation = store::start_conversation(&b.pool, repo.id, BRANCH)
        .await
        .unwrap()
        .expect("the Repo was just registered");
    assert_eq!(conversation, THERE);

    a.linked_to(&b.device, vec![b.at()]).await;
    b.linked_to(&a.device, vec![a.at()]).await;

    (a, b)
}

/// Where a local call stands when it is for `device` instead: the prefix takes
/// the place of `/api/ui`, and everything under it is untouched.
fn through(device: &str, path: &str) -> String {
    let leaf = path
        .strip_prefix("/api/ui")
        .expect("a relayed call is a call into the viewer's own namespace");

    format!("/api/ui/members/{device}{leaf}")
}

/// Make `request` of `app`, and hand back what it answered.
async fn fetch(app: &Router, request: Request<Body>) -> (StatusCode, String) {
    let answered = app.clone().oneshot(request).await.unwrap();
    let status = answered.status();
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();

    (status, String::from_utf8_lossy(&bytes).into_owned())
}

/// A read, made the way the browser makes one.
async fn get(app: &Router, path: &str) -> (StatusCode, String) {
    fetch(
        app,
        Request::builder().uri(path).body(Body::empty()).unwrap(),
    )
    .await
}

/// And a press, with a JSON body.
async fn post<T: DeserializeOwned>(app: &Router, path: &str, body: &serde_json::Value) -> T {
    let body = serde_json::to_vec(body).unwrap();

    let (status, said) = fetch(
        app,
        Request::builder()
            .method("POST")
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONTENT_LENGTH, body.len())
            .body(Body::from(body))
            .unwrap(),
    )
    .await;

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
fn answered(comment: &str) -> Response {
    Response {
        answers: Vec::<Answer>::new(),
        comment: Some(comment.to_owned()),
        direction: None,
        nothing_else: false,
    }
}

/// A read through the hop answers what the far end's own workbench answers, and
/// a press through it settles over there.
///
/// The two together, because the criterion is about both and they are one
/// claim: what stands at the far end of this hop is B's own namespace over B's
/// own store, so what comes back is what B would have said and what is pressed
/// is what B has done.
#[tokio::test]
async fn a_read_and_a_press_under_a_members_id_are_that_members_own() {
    let (a, b) = linked().await;

    let set = store::ask(
        &b.pool,
        THERE,
        &bare("what shall we call it?"),
        store::Ask::Blocking,
    )
    .await
    .unwrap()
    .expect("the Conversation is there to ask from");

    let (status, relayed) = get(
        &a.workbench(),
        &through(B, &format!("/api/ui/conversations/{THERE}")),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "reading B's Conversation: {relayed}"
    );
    assert!(
        relayed.contains(BRANCH),
        "the view should be B's Conversation, got:\n{relayed}",
    );

    let (status, bs_own) = get(
        &router_keeping(b.pool.clone(), b.dir.path().to_owned()),
        &format!("/api/ui/conversations/{THERE}"),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "B reading its own: {bs_own}");
    assert_eq!(
        relayed, bs_own,
        "and what came back through the hop is what B answered its own browser, \
         byte for byte",
    );

    let outcome: Submitted = post(
        &a.workbench(),
        &through(B, &format!("/api/ui/sets/{}/response", set.id)),
        &serde_json::to_value(answered("through the hop")).unwrap(),
    )
    .await;

    assert_eq!(outcome, Submitted::Accepted, "the press is B's own answer");

    let stored = store::load_response(&b.pool, set.id)
        .await
        .unwrap()
        .expect("the Set was answered through the hop, so B holds a Response for it");

    assert_eq!(
        stored.response.comment.as_deref(),
        Some("through the hop"),
        "and it settled on B rather than anywhere on A",
    );

    // Nothing of this went into A's own store: the hop is a call carried, not a
    // copy kept.
    let view: ConversationView =
        serde_json::from_str(&relayed).expect("the relayed answer is a Conversation view");
    assert_eq!(view.branch, BRANCH);

    let (status, _) = get(&a.workbench(), &format!("/api/ui/conversations/{THERE}")).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "A has no Conversation of its own, which is what makes the read above a relay",
    );
}

/// An attachment goes through, and lands as a file on the far end's disk.
#[tokio::test]
async fn an_attachment_is_written_on_the_device_that_holds_the_conversation() {
    let (a, b) = linked().await;
    let file = vec![b'x'; 64 * 1024];

    let (status, said) = fetch(
        &a.workbench(),
        Request::builder()
            .method("POST")
            .uri(through(
                B,
                &format!("/api/ui/conversations/{THERE}/attachments/notes.txt"),
            ))
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_LENGTH, file.len())
            .body(Body::from(file.clone()))
            .unwrap(),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "attaching through the hop: {said}");

    let attached: Attached = serde_json::from_str(&said).unwrap();
    let Attached::Attached { attachment } = attached else {
        panic!("expected the file to be attached, got {attached:?}");
    };
    assert_eq!(attachment.name, "notes.txt");

    let kept = b
        .dir
        .path()
        .join("attachments")
        .join(THERE.to_string())
        .join(&attachment.name);

    assert_eq!(
        std::fs::read(&kept).unwrap(),
        file,
        "the bytes are on B's disk at {}, which is where the far end put them",
        kept.display(),
    );
}

/// And a body over the far end's own limit comes back as the far end's own
/// `413` — without either end of the hop having held it.
///
/// **The body is endless as far as the hop is concerned**, which is what makes
/// this a test of the streaming rather than of the limit: a hop that read the
/// browser's body before dialling would still be reading when this assertion
/// wants an answer, and there would be no answer to make. What ends it is B —
/// its route carries a limit of its own, it stops reading at that limit and
/// answers `413`, and the answer comes back up the hop while the browser is
/// still sending.
///
/// So the count of what the browser managed to send is the second half of the
/// claim: it is a fraction of what it offered, because nothing along the way
/// was waiting for the end of it.
#[tokio::test]
async fn a_body_over_the_far_ends_limit_is_the_far_ends_own_refusal() {
    let (a, _b) = linked().await;

    // Four times the far end's limit, which nothing here intends to send: it is
    // how much this stream *would* go on producing, and the point is that it
    // does not have to.
    let offering = 4 * MAX_BYTES;
    let sent = Arc::new(AtomicUsize::new(0));

    let (status, said) = fetch(
        &a.workbench(),
        Request::builder()
            .method("POST")
            .uri(through(
                B,
                &format!("/api/ui/conversations/{THERE}/attachments/enormous.bin"),
            ))
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_LENGTH, offering)
            .body(Body::from_stream(Endless {
                left: offering,
                sent: Arc::clone(&sent),
            }))
            .unwrap(),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "the far end's own refusal is what comes back, got: {said}",
    );

    let sent = sent.load(Ordering::SeqCst);

    assert!(
        sent >= MAX_BYTES,
        "the far end reads up to its own limit before it refuses, so the browser \
         should have got at least {MAX_BYTES} bytes out; it sent {sent}, which is a \
         refusal made somewhere short of B",
    );

    assert!(
        sent < offering / 2,
        "the hop answered after {sent} bytes of the {offering} on offer, which is \
         what a body carried through rather than held looks like",
    );
}

/// A body that goes on until somebody stops reading it, counting what it gave.
struct Endless {
    /// What is left of what it would produce, in bytes.
    left: usize,

    /// And what it has produced, which is the whole of what this is for.
    sent: Arc<AtomicUsize>,
}

impl Stream for Endless {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.left == 0 {
            return Poll::Ready(None);
        }

        let chunk = self.left.min(64 * 1024);

        self.left -= chunk;
        self.sent.fetch_add(chunk, Ordering::SeqCst);

        Poll::Ready(Some(Ok(Bytes::from(vec![b'x'; chunk]))))
    }
}

/// The three Device Ids that are not dialled are each refused by name.
///
/// Together, because what makes each of them worth anything is the other two:
/// a hop that answered its own id would be a second spelling of every local
/// call, one that missed a stranger's id would be saying the path was wrong,
/// and one that hung on a member that is not there would be the browser
/// waiting on a machine with its lid shut.
#[tokio::test]
async fn the_ids_that_are_not_dialled_are_refused_by_name() {
    let (a, _b) = linked().await;

    let (gone, _held) = a_device_called(GONE);
    a.linked_to(&gone, vec![nothing_there()]).await;

    let app = a.workbench();
    let asking = "/api/ui/conversations";

    let (status, said) = get(&app, &through(A_STRANGER, asking)).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "a device this one is not linked to: {said}",
    );
    assert!(
        said.contains(A_STRANGER) && said.contains("knows no device"),
        "the refusal names the device and says this one has never heard of it, \
         got:\n{said}",
    );

    let (status, said) = get(&app, &through(A, asking)).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "this device's own id: {said}",
    );
    assert!(
        said.contains(A) && said.contains("is this device"),
        "the refusal says the id is this device's own, got:\n{said}",
    );

    let (status, said) = get(&app, &through(GONE, asking)).await;
    assert_eq!(
        status,
        StatusCode::BAD_GATEWAY,
        "a member that is not switched on: {said}",
    );
    assert!(
        said.contains(GONE) && said.contains("answered at none of the addresses"),
        "the refusal names the device and that nothing answered for it, got:\n{said}",
    );
}
