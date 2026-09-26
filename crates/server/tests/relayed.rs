//! The far end of a relay hop: the viewer's own namespace answered on the Peer
//! Listener, behind the Member Gate rather than behind the Workbench Key's
//! (ADR-0020, *The opened device relays*).
//!
//! **Every call here is a real dial over a real socket with a real handshake**,
//! for the reason `tests/peer.rs` makes every one of its own: what admits a
//! caller on this listener is the certificate it presented, and a router asked
//! in process is a namespace with no gate in front of it at all. So this file
//! stands the listener up on the loopback and dials it as the two callers the
//! acceptance criteria name — a member, and a device this one holds no
//! membership for.
//!
//! **And nothing here sends a cookie.** The client below has no cookie store and
//! is given no header: what gets a member through is the handshake, and a
//! member's own Workbench Key never leaves the device it was issued on. The same
//! request is put to a *keyed* workbench router beside it, holding nothing, to
//! show that what answers here is not merely a server with its gate off.
//!
//! What is asked of the namespace itself — that a Set submitted through it
//! settles, that the rendering is the rendering — is `tests/ui_api.rs`'s and
//! `tests/ui_content.rs`'s, and is not asked twice: this is one router of routes
//! mounted twice, so what is worth proving here is *which caller reaches it* and
//! *which of it is held back*.

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use tower::ServiceExt;
use verkstead_server::device::Device;
use verkstead_server::device::reading::Reading;
use verkstead_server::key::WorkbenchKey;
use verkstead_server::nudge::Nudges;
use verkstead_server::peer::{self, Members};
use verkstead_server::platform::Platform;
use verkstead_server::remote::Tailscale;
use verkstead_server::{open_database, router_keyed, router_over_the_link, store};
use verkstead_store::{Linking, record_member};

/// The device whose workbench is reached through the link: the one standing the
/// listener up.
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The device that reaches it, written down as a member of this one's cluster.
const A_MEMBER: &str = "0011223344556677889900aabbccddee";

/// And one this device holds no membership for, which is every other device.
const A_STRANGER: &str = "ffeeddccbbaa00998877665544332211";

/// The branch the one Conversation in the store is on, so that what comes back
/// over the link is assertable as something rather than merely as a 200.
const BRANCH: &str = "solid-viewer";

/// The list a member reads, which is the criterion's own path.
const CONVERSATIONS: &str = "/api/ui/conversations";

/// And a path under that namespace no route answers, which is refused as a
/// membership rather than missed: a caller with no membership has no business
/// being told which of this device's endpoints exist.
const NOTHING_ANSWERS_THIS: &str = "/api/ui/nothing-answers-this";

/// Two devices linked, and the workbench one of them is serving.
///
/// The store is the serving device's own, and both routers are over it: the
/// Peer Listener's, which is what this file dials, and the keyed workbench one,
/// which is what the same question is put to holding nothing.
struct Linked {
    /// Where the Peer Listener landed. The operating system's port rather than
    /// 8423, for the reason `tests/peer.rs` takes one: two of these running at
    /// once must not fight each other.
    address: SocketAddr,

    /// The device doing the reaching, whose certificate is the whole of how it
    /// is admitted.
    member: Device,

    /// And one nothing here has recorded.
    stranger: Device,

    /// The workbench the serving device answers its own browser out of, keyed
    /// the way the served router is.
    workbench: Router,

    /// The key that router stands on, which the browser holds and the member
    /// does not.
    key: WorkbenchKey,

    /// Held for the length of the test: three identities and a database live in
    /// them, and a directory dropped early is a certificate gone out from under
    /// a listener still presenting it.
    _dirs: Vec<tempfile::TempDir>,
}

impl Linked {
    async fn stood_up() -> Linked {
        let dir = tempfile::tempdir().unwrap();
        let pool = open_database(&dir.path().join("verkstead.db"))
            .await
            .unwrap();

        // One Repo and one Conversation on it, so that the list the criterion
        // names has a row in it: a namespace answering `[]` through the link
        // would be a namespace that had not reached the store.
        let repo = store::register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
            .await
            .unwrap()
            .expect("nothing is registered at that path yet");

        store::start_conversation(&pool, repo.id, BRANCH)
            .await
            .unwrap()
            .expect("the Repo was just registered");

        let device = Device::stated(dir.path(), THIS_DEVICE).unwrap();

        let (member, held) = a_device_called(A_MEMBER);
        let (stranger, somewhere) = a_device_called(A_STRANGER);

        // The row a join writes, written by hand: what a membership comes from
        // is `tests/joining.rs`'s, and what it *admits* is this file's.
        record_member(
            &pool,
            &Linking {
                device: member.id().to_owned(),
                name: "somewhere-else".to_owned(),
                os: "Linux".to_owned(),
                addresses: vec!["192.168.1.31".to_owned()],
                fingerprint: member.fingerprint().to_owned(),
            },
        )
        .await
        .unwrap();

        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");
        let address = listener.address();

        // The router this stage adds, behind the real gate: the viewer's own
        // namespace over this device's state, and none of the workbench's other
        // halves.
        tokio::spawn(listener.serving(peer::router(
            device,
            nowhere(),
            Members::recorded(pool.clone()),
            peer::joining::Joins::none(),
            Nudges::new(),
            router_over_the_link(pool.clone(), dir.path().to_owned()),
        )));

        let key = WorkbenchKey::stated(dir.path(), "the-workbench-key").unwrap();

        Linked {
            address,
            member,
            stranger,
            workbench: router_keyed(pool, key.clone()),
            key,
            _dirs: vec![dir, held, somewhere],
        }
    }
}

/// A device standing on its own two files, whatever this suite wants to call it.
fn a_device_called(id: &str) -> (Device, tempfile::TempDir) {
    let elsewhere = tempfile::tempdir().unwrap();
    let device = Device::stated(elsewhere.path(), id).unwrap();

    (device, elsewhere)
}

/// A machine with no Tailscale and no interfaces, which is what the identity
/// endpoint would read and nothing here asks about: the reading is the one thing
/// on this listener that would otherwise be whatever box the suite is running
/// on.
fn nowhere() -> Reading {
    Reading::stated(
        Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422),
        Platform::HERE,
        None,
        Vec::new(),
    )
}

/// What a caller shows the listener.
///
/// A borrow of the device rather than the device, so that one caller can make
/// two dials: what a client is built from is the certificate on disk, and two
/// calls by one member are two clients over one identity.
#[derive(Clone, Copy)]
enum Showing<'a> {
    /// A certificate, which is the whole of who a caller is here.
    A(&'a Device),

    /// And no client certificate at all.
    Nothing,
}

/// Dial the listener and ask for `path`, holding no cookie of any kind.
///
/// reqwest with a preconfigured TLS configuration, which is the client this
/// device's own dials are made with — see `peer::dialling`. Whatever the server
/// shows is taken, the way a device linking for the first time takes it: there
/// is no certificate authority anywhere in a cluster to check one against.
async fn asked(linked: &Linked, showing: Showing<'_>, path: &str) -> (u16, String) {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let algorithms = provider.signature_verification_algorithms;

    let dialling = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(WhateverTheServerShows { algorithms }));

    let dialling = match showing {
        Showing::Nothing => dialling.with_no_client_auth(),
        Showing::A(device) => {
            let pem = device.certificate().as_bytes();

            dialling
                .with_client_auth_cert(
                    vec![CertificateDer::from_pem_slice(pem).unwrap()],
                    PrivateKeyDer::from_pem_slice(pem).unwrap(),
                )
                .expect("a device's own certificate and the key that signed it")
        }
    };

    let answered = reqwest::Client::builder()
        .use_preconfigured_tls(dialling)
        .build()
        .unwrap()
        .get(format!("https://{}{path}", linked.address))
        .send()
        .await
        .expect("the handshake completes and the listener answers");

    let status = answered.status().as_u16();

    (status, answered.text().await.unwrap())
}

/// And the same question put to the workbench's own router, as a browser
/// holding `cookie` would put it.
async fn over_the_workbench(app: &Router, path: &str, cookie: Option<&str>) -> (u16, String) {
    let mut request = Request::builder().uri(path);

    if let Some(cookie) = cookie {
        request = request.header(axum::http::header::COOKIE, cookie);
    }

    let answered = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();

    let status = answered.status().as_u16();
    let bytes = answered.into_body().collect().await.unwrap().to_bytes();

    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

/// The whole of what this task claims: a member's call over the Peer Listener
/// answers what the workbench's own does, with no key cookie sent and none
/// needed.
///
/// The three answers together, because the claim is about all three: the
/// workbench answers a browser holding the key, refuses the same request from
/// one holding nothing, and the Peer Listener answers the member — holding
/// nothing — with the very bytes the browser got.
#[tokio::test]
async fn a_members_call_answers_what_the_workbenchs_own_does() {
    let linked = Linked::stood_up().await;

    let (status, browsers) =
        over_the_workbench(&linked.workbench, CONVERSATIONS, Some(&linked.key.cookie())).await;

    assert_eq!(status, 200, "the browser holding the key reads the list");
    assert!(
        browsers.contains(BRANCH),
        "the list should carry the Conversation in the store, got:\n{browsers}",
    );

    let (status, _) = over_the_workbench(&linked.workbench, CONVERSATIONS, None).await;

    assert_eq!(
        status, 401,
        "and a browser holding nothing is refused, which is what makes the answer \
         below an answer rather than a server with its gate off",
    );

    let (status, members) = asked(&linked, Showing::A(&linked.member), CONVERSATIONS).await;

    assert_eq!(
        status, 200,
        "a member reaches the namespace over the link it already holds, got:\n{members}",
    );
    assert_eq!(
        members, browsers,
        "and reads exactly what the browser read: one router of routes, mounted \
         twice over one state",
    );
}

/// And the two callers that are not members are refused in the gate's own
/// words — for a path that is a route and for one that is not alike.
///
/// The second half is what makes the first worth anything on this listener: a
/// caller with no membership has no business learning which of this device's
/// endpoints exist, so a path nothing answers is refused here rather than
/// missed.
#[tokio::test]
async fn a_caller_that_is_no_member_is_refused_by_the_gate() {
    let linked = Linked::stood_up().await;

    for (named, showing) in [
        (
            "a device this one has never recorded",
            Showing::A(&linked.stranger),
        ),
        ("a caller showing nothing at all", Showing::Nothing),
    ] {
        for path in [CONVERSATIONS, NOTHING_ANSWERS_THIS] {
            let (status, said) = asked(&linked, showing, path).await;

            assert_eq!(
                status, 403,
                "{named} holds no membership here, and {path} is a member's or is \
                 refused, got:\n{said}",
            );
            assert!(
                said.contains("not a member"),
                "and the refusal should be the gate's own words rather than a miss \
                 a caller cannot act on, got:\n{said}",
            );
        }
    }
}

/// And the three prefixes this device keeps to itself are refused by name, for
/// a member.
///
/// **Which is what makes *a member's Workbench Key never leaves it* a fact about
/// the mechanism**: the Remote access reading carries the login link with that
/// key on it, and a namespace served whole would hand it to whoever holds the
/// other device's cookie.
///
/// Each of them twice — the path a route answers today, and a path under it
/// nothing answers — because the refusal is about the namespace rather than
/// about the endpoints that happen to be in it. And the refusal says which it
/// is: a member that could not tell this from a missing route could not tell a
/// namespace kept back from a Verkstead too old to have the endpoint.
#[tokio::test]
async fn the_prefixes_this_device_keeps_to_itself_are_refused_by_name() {
    let linked = Linked::stood_up().await;

    for path in [
        "/api/ui/remote",
        "/api/ui/remote/serve",
        "/api/ui/remote/nothing-answers-this",
        "/api/ui/devices",
        "/api/ui/devices/discovered",
        "/api/ui/devices/nothing-answers-this",
        "/api/ui/push/key",
        "/api/ui/push/nothing-answers-this",
    ] {
        let (status, said) = asked(&linked, Showing::A(&linked.member), path).await;

        assert_eq!(
            status, 403,
            "{path} is this device's own and is not served over its peer listener, \
             got:\n{said}",
        );
        assert!(
            said.contains("keeps to itself"),
            "and the refusal should name what it is rather than read as a membership \
             or a miss, got:\n{said}",
        );
    }
}

/// And a path that merely begins with one of those words is not held back by its
/// spelling.
///
/// The prefixes are whole segments — the pane's own reading and everything under
/// it — so a namespace somebody adds later whose name starts with the same
/// letters is refused by the gate or missed, rather than answered with a
/// sentence about a namespace it is not under.
#[tokio::test]
async fn a_path_that_merely_starts_with_one_of_them_is_not_that_namespace() {
    let linked = Linked::stood_up().await;

    let (status, said) = asked(&linked, Showing::A(&linked.member), "/api/ui/remotes").await;

    assert!(
        !said.contains("keeps to itself"),
        "`/api/ui/remotes` is not a path under `/api/ui/remote/`, got {status}:\n{said}",
    );
}

/// The agents' half, the health check and the workbench's own pages are not on
/// this listener at all.
///
/// A session's Conversation-scoped API answers the loopback and the named pipe,
/// which is all a session ever dials; whether this server is up is nobody's
/// Conversation; and a page is what a browser asks its *own* device for. None of
/// the three is a route on the relayed router, so a member asking for one is
/// answered by no route — which is not the same as being answered.
#[tokio::test]
async fn the_agents_half_and_the_pages_are_not_on_this_listener() {
    let linked = Linked::stood_up().await;

    for path in [
        "/api/v1/health",
        "/conversations/1/api/v1/sets",
        "/",
        "/conversations",
    ] {
        let (status, said) = asked(&linked, Showing::A(&linked.member), path).await;

        assert_ne!(
            status, 200,
            "{path} is not a member's to ask of this listener, got:\n{said}",
        );
    }
}

/// And it is the store that answers, rather than a namespace standing over one
/// of its own.
///
/// A row written after the listener came up is read through the link, which is
/// what says this is the device's own workbench rather than a snapshot of it —
/// and what the relay of the next task is forwarding to.
#[tokio::test]
async fn what_answers_is_this_devices_own_store() {
    let linked = Linked::stood_up().await;

    let (status, said) = asked(&linked, Showing::A(&linked.member), CONVERSATIONS).await;

    assert_eq!(status, 200, "got:\n{said}");
    assert!(
        said.contains(BRANCH),
        "the Conversation in the store is read through the link, got:\n{said}",
    );
}

/// The client half of *whatever arrives is taken*: there is no certificate
/// authority anywhere in a cluster, so a dial takes what the far end shows and
/// what proves it is the fingerprint compared afterwards — which on this
/// listener the member row already holds.
///
/// Said here rather than shared with `tests/peer.rs`, which has one of its own:
/// what the two suites have in common is a client, and a helper crate between
/// two test files would be a third place to read before either of them made
/// sense.
#[derive(Debug)]
struct WhateverTheServerShows {
    algorithms: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for WhateverTheServerShows {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}
