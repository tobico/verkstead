//! The listener devices talk to each other over: what a caller meets when it
//! dials the peer port (ADR-0020).
//!
//! Every test here is a real dial over a real socket with a real handshake,
//! because that is the whole of what this stage claims: a router asked in
//! process would answer the identity endpoint whether or not a certificate had
//! ever been presented, and whether a certificate is *asked for* is a fact
//! about the handshake and about nothing else.
//!
//! The two callers are the two the acceptance criteria name: one with nothing
//! to show, and one showing a certificate this device has never heard of. The
//! second is another Verkstead's identity rather than a certificate minted for
//! the occasion — what an unknown device presents is exactly what a device
//! presents, and a stranger made some other way would be a stranger of a shape
//! no peer will ever be.
//!
//! The same two are what the member gate is asked about, because a membership
//! is what neither of them has: what the identity endpoint answers them, what
//! the routes behind the gate refuse them with, and that the refusal is read for
//! what it says as much as for its status. The join post is the one path on this
//! listener the gate deliberately does not stand over — a join comes from a
//! non-member by definition — so what is asked of it here is that the gate is
//! not what answers it, and what it *does* is `tests/joining.rs`'s.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::ConnectInfo;
use axum::routing::get;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, RENEW_WITHIN};
use verkstead_server::open_database;
use verkstead_server::peer;
use verkstead_server::platform::{self, Platform};
use verkstead_server::remote::Tailscale;
use verkstead_store::{Linking, forget_member, record_member};

/// The id this device is stated as, so that what a test asserts against is a
/// string it chose rather than sixteen random bytes it has to filter out of a
/// payload — see [`Device::stated`], which is here for that reason.
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the one the stranger below is stated as. A device this one has never
/// been linked to.
const SOME_OTHER_DEVICE: &str = "0011223344556677889900aabbccddee";

/// And a third, for the suites about the member gate: one device is written
/// down as a member and the other is not, which is what the gate tells apart.
const A_THIRD_DEVICE: &str = "ffeeddccbbaa00998877665544332211";

/// The join post, which now stands *outside* the gate: a join comes from a
/// device this one holds no membership for, that being what a join is.
///
/// Here to be asked the opposite question of the one below — that the gate is
/// not what answers a caller reaching for it. What the post itself does is
/// `tests/joining.rs`'s, which presses Add on one Verkstead and reads the
/// request off another.
const THE_JOIN: &str = "/api/peer/v1/join";

/// And a path no stage will ever answer, which on this listener is refused as a
/// membership rather than missed: a caller with no membership has no business
/// being told which of this device's endpoints exist.
const NOTHING_ANSWERS_THIS: &str = "/api/peer/v1/nothing-answers-this";

/// A peer listener up on the loopback, with the device it is presenting.
///
/// The port is the operating system's rather than 8423: a suite that took the
/// real one would fight whatever is already on it, and two of these tests
/// running at once would fight each other.
struct Listening {
    address: SocketAddr,
    device: Device,

    /// Held for the length of the test, because the identity lives in it: a
    /// directory dropped early is a certificate gone out from under the
    /// listener still presenting it.
    _dir: tempfile::TempDir,
}

impl Listening {
    /// Stand one up serving what a Verkstead serves, until the test is over.
    ///
    /// Reading a machine with no Tailscale and no interfaces, which is what
    /// every test here but the ones about the addresses wants: the tailnet half
    /// would otherwise be whatever the box running the suite is on, and running
    /// somebody's real `tailscale` to answer a test about a fingerprint would be
    /// a suite asking a question it is not about.
    fn with_the_device_called(id: &str) -> Listening {
        Listening::reading(id, nowhere())
    }

    /// And one whose device is on the machine `reading` describes, for the
    /// tests that are about what this device says of the machine it is on.
    fn reading(id: &str, reading: Reading) -> Listening {
        Listening::serving(id, |device| {
            peer::router(
                device,
                reading,
                peer::Members::none(),
                peer::joining::Joins::none(),
                verkstead_server::nudge::Nudges::new(),
                Router::new(),
            )
        })
    }

    /// And one serving `answering` instead, for the questions that are about
    /// what a route on this listener can read rather than about which routes
    /// there are.
    fn serving(id: &str, answering: impl FnOnce(Device) -> Router) -> Listening {
        let dir = tempfile::tempdir().unwrap();
        let device = Device::stated(dir.path(), id).unwrap();

        Listening::standing(dir, device, answering)
    }

    /// And one whose device came up in the middle of a changeover: a
    /// certificate near enough its expiry for the start to make another, and a
    /// member that has yet to acknowledge the one it made.
    ///
    /// Which is the whole of what this is here to ask — the listener presents
    /// the certificate that member holds, rather than the one it has not been
    /// told about yet, and a changeover is a re-issue that costs no call.
    async fn mid_changeover(id: &str) -> Listening {
        let dir = tempfile::tempdir().unwrap();

        Device::stated_good_for(dir.path(), id, RENEW_WITHIN - time::Duration::days(1)).unwrap();

        let device = Device::issued(dir.path(), &peer::Members::stated(1))
            .await
            .unwrap();

        assert!(
            device.incoming_fingerprint().is_some(),
            "a start inside the renewal window makes a fresh certificate",
        );

        Listening::standing(dir, device, |device| {
            peer::router(
                device,
                nowhere(),
                peer::Members::stated(1),
                peer::joining::Joins::none(),
                verkstead_server::nudge::Nudges::new(),
                Router::new(),
            )
        })
    }

    /// And one that lets go of a caller who has not got through the handshake
    /// in `handshake`, rather than in the ten seconds a running server gives
    /// one.
    ///
    /// A stated deadline for the reason the certificate's life is stated: what
    /// is being asked is that the connection is let go of at all, and a test
    /// that waited out the real one would be a test spending ten seconds on the
    /// clock rather than on the question.
    fn letting_go_after(id: &str, handshake: Duration) -> Listening {
        let dir = tempfile::tempdir().unwrap();
        let device = Device::stated(dir.path(), id).unwrap();

        Listening::within(
            dir,
            device,
            |device| {
                peer::router(
                    device,
                    nowhere(),
                    peer::Members::none(),
                    peer::joining::Joins::none(),
                    verkstead_server::nudge::Nudges::new(),
                    Router::new(),
                )
            },
            Some(handshake),
        )
    }

    /// The socket, the handshakes and the serve, over a device that is already
    /// made: what every constructor above comes down to once it has said which
    /// device it is standing behind.
    fn standing(
        dir: tempfile::TempDir,
        device: Device,
        answering: impl FnOnce(Device) -> Router,
    ) -> Listening {
        Listening::within(dir, device, answering, None)
    }

    /// The same, with the handshake's deadline said where a suite is asking
    /// about that — see [`Listening::letting_go_after`].
    fn within(
        dir: tempfile::TempDir,
        device: Device,
        answering: impl FnOnce(Device) -> Router,
        handshake: Option<Duration>,
    ) -> Listening {
        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");

        let listener = match handshake {
            Some(handshake) => listener.handshaking_within(handshake),
            None => listener,
        };

        let address = listener.address();

        tokio::spawn(listener.serving(answering(device.clone())));

        Listening {
            address,
            device,
            _dir: dir,
        }
    }
}

/// What a caller shows the listener: nothing, or somebody else's identity.
enum Showing {
    /// No client certificate at all, which is what a device asking *who are
    /// you* has: it is asking because it does not know yet, and a link is not
    /// what it is reaching for.
    Nothing,

    /// A certificate, from a device this one holds no membership for — which
    /// in this stage is every device.
    A(Device),
}

/// How a caller dials: taking whatever the server shows, and showing `showing`
/// of its own.
///
/// Whatever the server shows is taken, the way a device linking for the first
/// time takes it: what proves the far end is the certificate compared against a
/// fingerprint afterwards, and there is no certificate authority anywhere in a
/// cluster to check one against.
fn dialling(showing: Showing) -> ClientConfig {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let algorithms = provider.signature_verification_algorithms;

    let dialling = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(WhateverTheServerShows { algorithms }));

    match showing {
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
    }
}

/// Dial `listening`, ask for `path`, and hand back what came of it: the
/// certificate the server presented, and the response as it arrived.
async fn asking(
    listening: &Listening,
    showing: Showing,
    path: &str,
) -> (CertificateDer<'static>, String) {
    // Dialled by the name the certificate is made out to, which is the device
    // id — that is what a peer will have to hand, and the only name this
    // certificate has.
    let name = ServerName::try_from(listening.device.id().to_owned()).unwrap();

    let connection = TcpStream::connect(listening.address).await.unwrap();
    let mut secured = TlsConnector::from(Arc::new(dialling(showing)))
        .connect(name, connection)
        .await
        .expect("the handshake should complete");

    let presented = secured
        .get_ref()
        .1
        .peer_certificates()
        .expect("a server presents its certificate")[0]
        .clone()
        .into_owned();

    // `Connection: close`, so that reading to the end of the stream is reading
    // to the end of the response and this suite needs no HTTP client of its
    // own.
    secured
        .write_all(
            format!(
                "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                listening.device.id(),
            )
            .as_bytes(),
        )
        .await
        .unwrap();

    let mut answered = Vec::new();
    secured.read_to_end(&mut answered).await.unwrap();

    (presented, String::from_utf8(answered).unwrap())
}

/// The same dial where what is expected is that it does not work: everything
/// `asking` does, with the failure handed back instead of unwrapped.
///
/// Read to the end rather than stopped at the connect, because a TLS 1.3 client
/// is finished with its side of the handshake the moment it has sent its
/// certificate — the server's refusal is an alert that arrives after, so it is
/// the read that meets it rather than the connect.
async fn turned_away(listening: &Listening, showing: Showing) -> String {
    let name = ServerName::try_from(listening.device.id().to_owned()).unwrap();

    let connection = TcpStream::connect(listening.address).await.unwrap();

    let mut secured = match TlsConnector::from(Arc::new(dialling(showing)))
        .connect(name, connection)
        .await
    {
        Ok(secured) => secured,
        Err(why) => return why.to_string(),
    };

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        peer::IDENTITY,
        listening.device.id(),
    );

    if let Err(why) = secured.write_all(request.as_bytes()).await {
        return why.to_string();
    }

    let mut answered = Vec::new();

    match secured.read_to_end(&mut answered).await {
        Err(why) => why.to_string(),
        Ok(_) if answered.is_empty() => "the connection was closed".to_owned(),
        Ok(_) => panic!(
            "the caller should have been turned away, and was answered:\n{}",
            String::from_utf8_lossy(&answered),
        ),
    }
}

/// The body of a response read that way: everything past the blank line.
fn body(answered: &str) -> &str {
    answered
        .split_once("\r\n\r\n")
        .expect("a response has a head and a body")
        .1
}

/// And its status, which is what says a refusal apart from a miss.
fn status(answered: &str) -> u16 {
    answered
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("a response starts with a status line, got:\n{answered}"))
}

/// A certificate's fingerprint in the spelling the device module prints one:
/// the SHA-256 of its own bytes, upper-case hex in colon-separated pairs.
///
/// Worked out here rather than read off the handle, because what this suite is
/// checking is that the certificate that came down the wire is the one the
/// startup line named — and taking the string off the handle at both ends
/// would compare it with itself.
fn fingerprint_of(certificate: &CertificateDer<'_>) -> String {
    Sha256::digest(certificate)
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<String>>()
        .join(":")
}

/// The client half of *whatever arrives is taken*: a device linking for the
/// first time has nothing to check the far end against but the fingerprint it
/// is about to read, so it takes the certificate and compares afterwards.
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

#[tokio::test]
async fn a_caller_with_no_certificate_of_its_own_reads_the_identity() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);

    let (_, answered) = asking(&listening, Showing::Nothing, peer::IDENTITY).await;

    let identity: serde_json::Value = serde_json::from_str(body(&answered))
        .unwrap_or_else(|why| panic!("the identity should be JSON: {why}\n{answered}"));

    assert_eq!(
        identity["device"], THIS_DEVICE,
        "a device that has never been heard of is exactly who asks this — it is asking \
         because it does not know yet, got:\n{answered}",
    );
}

/// The demonstration the stage is judged on: a caller reads an id over TLS, and
/// the certificate the handshake handed it is the one the startup line printed.
#[tokio::test]
async fn the_certificate_the_handshake_hands_over_is_the_one_the_startup_line_printed() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);

    let (presented, answered) = asking(&listening, Showing::Nothing, peer::IDENTITY).await;

    assert_eq!(
        fingerprint_of(&presented),
        listening.device.fingerprint(),
        "what is presented has to be the device's own certificate, or a link pinned on \
         the printed fingerprint would be pinned on nothing",
    );

    let identity: serde_json::Value = serde_json::from_str(body(&answered)).unwrap();

    assert_eq!(
        identity["fingerprint"],
        listening.device.fingerprint(),
        "and the answer names the same one, which is what a caller checks the \
         handshake against, got:\n{answered}",
    );
}

/// And the other half of that: over a changeover the certificate the handshake
/// hands over is the *outgoing* one, which is the one every member holds.
///
/// A re-issue that started presenting the new certificate the moment it made
/// one would take every link down until each member had been told — which is
/// the call a changeover exists not to cost, and the reason two certificates
/// are held rather than one replaced.
#[tokio::test]
async fn a_device_in_the_middle_of_a_changeover_presents_the_certificate_its_members_hold() {
    let listening = Listening::mid_changeover(THIS_DEVICE).await;

    let (presented, answered) = asking(&listening, Showing::Nothing, peer::IDENTITY).await;

    assert_eq!(
        fingerprint_of(&presented),
        listening.device.fingerprint(),
        "a member that has not acknowledged the new fingerprint is answered with the one \
         it holds",
    );
    assert_ne!(
        fingerprint_of(&presented),
        listening
            .device
            .incoming_fingerprint()
            .expect("this device is in the middle of a changeover"),
        "and the new certificate is not what is going out yet, or there would have been \
         nothing to announce",
    );

    let identity: serde_json::Value = serde_json::from_str(body(&answered)).unwrap();

    assert_eq!(
        identity["fingerprint"],
        listening.device.fingerprint(),
        "and what the endpoint names is the certificate the handshake just presented, \
         which is what a caller checks one against, got:\n{answered}",
    );
}

/// The other half of *asked for and not insisted on*: a certificate nothing
/// here has ever seen is taken at the handshake rather than refused at it. A
/// verifier that refused one is what the join of the stage after this could
/// never have got through.
#[tokio::test]
async fn a_caller_showing_an_unknown_certificate_reaches_the_identity_too() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);

    let elsewhere = tempfile::tempdir().unwrap();
    let stranger = Device::stated(elsewhere.path(), SOME_OTHER_DEVICE).unwrap();

    let (_, answered) = asking(&listening, Showing::A(stranger), peer::IDENTITY).await;

    let identity: serde_json::Value = serde_json::from_str(body(&answered))
        .unwrap_or_else(|why| panic!("the identity should be JSON: {why}\n{answered}"));

    assert_eq!(
        identity["device"], THIS_DEVICE,
        "the handshake must not be what decides which endpoints a caller reaches, \
         got:\n{answered}",
    );
}

/// Two callers at once, one of which never finishes its handshake: the other is
/// answered anyway.
///
/// This is why the accepting is a task per connection rather than a handshake
/// run inside `accept`. A peer port answers a LAN that may not be the human's
/// alone, and one socket opened and left silent would otherwise hold every
/// device behind it.
#[tokio::test]
async fn a_caller_that_opens_a_socket_and_says_nothing_holds_nobody_up() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);

    // Connected and then left alone: no ClientHello, so this connection's
    // handshake never completes.
    let _silent = TcpStream::connect(listening.address).await.unwrap();

    let (_, answered) = asking(&listening, Showing::Nothing, peer::IDENTITY).await;

    assert!(
        body(&answered).contains(THIS_DEVICE),
        "the silent connection should have held up its own task and nobody else's, \
         got:\n{answered}",
    );
}

/// And the other half of that: a caller that opens a socket and says nothing is
/// let go of rather than held on to for as long as it likes.
///
/// The test above proves such a caller holds nobody else up, which is a
/// different claim. Each of them costs this process a task and a descriptor for
/// as long as it is kept, and with no deadline on the handshake *how long* is
/// the caller's to decide — so a host on the LAN opens sockets until this
/// process is out of descriptors, and the workbench goes down with the listener,
/// the two being served out of one process.
///
/// What says the connection was let go of is the far end reading to its end: a
/// dropped connection closes the socket, so a read that had been waiting comes
/// back with nothing rather than waiting for ever.
#[tokio::test]
async fn a_caller_that_says_nothing_is_let_go_of() {
    let listening = Listening::letting_go_after(THIS_DEVICE, Duration::from_millis(100));

    let mut silent = TcpStream::connect(listening.address).await.unwrap();

    let mut nothing = [0u8; 1];
    let read = tokio::time::timeout(Duration::from_secs(10), silent.read(&mut nothing))
        .await
        .expect("the listener should have let go of a caller that never began a handshake");

    assert_eq!(
        read.unwrap(),
        0,
        "the connection should have been closed from the listener's end, which is what \
         gives the descriptor back",
    );
}

/// A device this one has never heard of, standing where a stranger stands.
///
/// The directory comes back with it because the identity lives in it: dropped
/// early, the certificate goes out from under the handle still presenting it.
fn a_stranger() -> (Device, tempfile::TempDir) {
    a_device_called(SOME_OTHER_DEVICE)
}

/// And a device standing on its own two files, whatever this suite wants to
/// call it — a member, or a second stranger beside the first.
fn a_device_called(id: &str) -> (Device, tempfile::TempDir) {
    let elsewhere = tempfile::tempdir().unwrap();
    let device = Device::stated(elsewhere.path(), id).unwrap();

    (device, elsewhere)
}

/// Where the probe router below says what the handshake handed it.
const PRESENTED: &str = "/presented";

/// A router that is nothing but a route reading its caller.
///
/// Stood up instead of the real one because the real one has nowhere to say
/// this: the identity endpoint answers a caller that has shown nothing, and
/// everything that *does* read a certificate is a stage away. What is being
/// checked is the shape — that the certificate the handshake took is there to
/// be read by the time a route runs — and a route of one line is the whole of
/// that question.
fn probing(_device: Device) -> Router {
    Router::new().route(
        PRESENTED,
        get(
            |ConnectInfo(caller): ConnectInfo<peer::Caller>| async move {
                caller.fingerprint().unwrap_or_else(|| "nothing".to_owned())
            },
        ),
    )
}

/// The first half of the gate's acceptance: a caller with nothing to show
/// reaches the identity endpoint — which the suite above proves — and, of the
/// routes the gate stands over, nothing at all.
#[tokio::test]
async fn a_caller_with_no_certificate_reaches_nothing_but_the_identity() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);

    let (_, answered) = asking(&listening, Showing::Nothing, NOTHING_ANSWERS_THIS).await;

    assert_eq!(
        status(&answered),
        403,
        "{NOTHING_ANSWERS_THIS} is a member's or is refused, and there is no member, \
         got:\n{answered}",
    );
}

/// And the second: a certificate nothing here has recorded completes the
/// handshake — the suite above proves that too — and is refused by every route
/// the gate stands over.
#[tokio::test]
async fn a_caller_showing_an_unknown_certificate_is_refused_by_the_gated_routes() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);
    let (stranger, _elsewhere) = a_stranger();

    let (_, answered) = asking(&listening, Showing::A(stranger), NOTHING_ANSWERS_THIS).await;

    assert_eq!(
        status(&answered),
        403,
        "a certificate is not a membership — the gate is what holds one, got:\n{answered}",
    );
}

/// And the join post is not one of those routes.
///
/// **Which is the whole arrangement** (ADR-0020, *The peer listener, and mutual
/// TLS*): a join comes from a device this one holds no membership for, that
/// being what a join *is*, so a gate over it would be a gate no link could ever
/// be made through. What is asked here is only that the gate is not what answers
/// a caller reaching for that path — a method this route does not take is turned
/// away by the route rather than refused as a stranger. What the post itself
/// does is `tests/joining.rs`'s.
#[tokio::test]
async fn the_join_post_is_not_behind_the_gate() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);
    let (stranger, _elsewhere) = a_stranger();

    let (_, answered) = asking(&listening, Showing::A(stranger), THE_JOIN).await;

    assert_ne!(
        status(&answered),
        403,
        "a device asking to be let in is a non-member by definition, and the gate \
         must not be what it meets, got:\n{answered}",
    );
}

/// What the refusal says, which is the thing a caller reads it for.
///
/// A device reaching for something on this listener has two ways of not getting
/// through: a Verkstead that will not have it, and a Verkstead too old to have
/// the route at all. It wants a human in the first case and an upgrade on the
/// other machine in the second, so a refusal that read as a missing path would
/// leave it unable to say which it had met.
#[tokio::test]
async fn the_refusal_says_it_is_a_membership_rather_than_a_missing_path() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);
    let (stranger, _elsewhere) = a_stranger();

    let (_, answered) = asking(&listening, Showing::A(stranger), NOTHING_ANSWERS_THIS).await;

    assert_ne!(
        status(&answered),
        404,
        "a refusal a caller cannot tell from a missing route is a refusal it \
         cannot act on, got:\n{answered}",
    );

    assert!(
        body(&answered).contains("not a member"),
        "the refusal should name what it is — a caller that is not a member of \
         this device's cluster, got:\n{answered}",
    );
}

/// The certificate the handshake accepted is readable by a route, which is
/// what the join of the stage after this pins into the pending request it
/// creates.
///
/// Read as its fingerprint rather than as the bytes, because that is the
/// comparison the member list and the human both make — and because a
/// fingerprint worked out on this side from the device's own file is a
/// different path to the same string, rather than the same string handed back.
#[tokio::test]
async fn a_route_can_read_the_certificate_the_handshake_took() {
    let listening = Listening::serving(THIS_DEVICE, probing);
    let (stranger, _elsewhere) = a_stranger();

    let (_, answered) = asking(&listening, Showing::A(stranger.clone()), PRESENTED).await;

    assert_eq!(
        body(&answered),
        stranger.fingerprint(),
        "the route should read the certificate the caller presented, got:\n{answered}",
    );

    let (_, answered) = asking(&listening, Showing::Nothing, PRESENTED).await;

    assert_eq!(
        body(&answered),
        "nothing",
        "and nothing where the caller presented nothing, that being an ordinary \
         caller rather than a failure, got:\n{answered}",
    );
}

/// A device whose certificate ran out a day ago is refused at the handshake.
///
/// This is the one thing the whole renewal rests on. A device makes its
/// certificate again before the ninety days are up *because* an expired one is
/// refused here — and nothing in a cluster checks a chain or asks an authority,
/// while a member list of fingerprints goes on matching a certificate for ever.
/// So if it is not refused at the handshake it is refused nowhere, and the
/// validity, the renewal window and the changeover are machinery holding
/// nothing up.
///
/// Stood at with a certificate made with a day of life *behind* it, which is
/// the same seam a start near the expiry is stood at with — a clock this
/// process does not keep is the alternative.
#[tokio::test]
async fn a_caller_whose_certificate_has_run_out_is_refused_at_the_handshake() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);

    let elsewhere = tempfile::tempdir().unwrap();
    let expired = Device::stated_good_for(
        elsewhere.path(),
        SOME_OTHER_DEVICE,
        -time::Duration::days(1),
    )
    .unwrap();

    turned_away(&listening, Showing::A(expired)).await;
}

/// And a device whose certificate is inside its validity still gets through,
/// which is what says the check above is about the dates rather than about
/// client certificates at all.
#[tokio::test]
async fn a_caller_whose_certificate_is_current_still_gets_through() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);
    let (stranger, _elsewhere) = a_stranger();

    let (_, answered) = asking(&listening, Showing::A(stranger), peer::IDENTITY).await;

    assert_eq!(
        identity(&answered)["device"],
        THIS_DEVICE,
        "a certificate this device has never seen is still a certificate, and the \
         handshake is not what decides who is a member, got:\n{answered}",
    );
}

/// The machine a device with nothing around it is on: no Tailscale, and no
/// interface worth advertising.
///
/// `verkstead-no-such-tailscale` is a program that is not there, which is what
/// a machine with no Tailscale on it *is* — see the Remote access suite, which
/// tells a missing binary from one that will not answer the same way.
fn nowhere() -> Reading {
    Reading::stated(
        Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422),
        Platform::HERE,
        None,
        Vec::new(),
    )
}

/// The same machine, on the two LAN addresses a test states for it.
fn on_the_lan() -> Reading {
    Reading::stated(
        Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422),
        Platform::HERE,
        None,
        vec!["192.168.1.24".parse().unwrap(), "10.0.0.7".parse().unwrap()],
    )
}

/// What a WSL kernel calls itself, which is the one thing that says one apart
/// from the Linux it is in every other way.
const WSL_KERNEL: &str = "5.15.167.4-microsoft-standard-WSL2";

/// The identity a caller reads, parsed.
fn identity(answered: &str) -> serde_json::Value {
    serde_json::from_str(body(answered))
        .unwrap_or_else(|why| panic!("the identity should be JSON: {why}\n{answered}"))
}

/// What a device says about the machine it is on: the hostname it is shown
/// under, and the word for its operating system.
///
/// The name is checked against the one reading there is of it rather than
/// against a string in this file: a hostname read off the box the suite is
/// running on would be a golden fixture nobody could commit, which is the same
/// reason the onboarding suite states the machines it asks about.
#[tokio::test]
async fn the_identity_names_the_machine_this_device_is_on() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);

    let (_, answered) = asking(&listening, Showing::Nothing, peer::IDENTITY).await;
    let identity = identity(&answered);

    assert_eq!(
        identity["name"],
        platform::hostname(),
        "the name is the hostname read off the machine, with nothing configurable \
         about it, got:\n{answered}",
    );

    assert!(
        !identity["name"].as_str().unwrap().is_empty(),
        "and a machine that will not say what it is called still reads as something, \
         got:\n{answered}",
    );

    assert_eq!(
        identity["os"],
        platform::os_word(Platform::HERE, platform::kernel_release().as_deref()),
        "and the OS is this platform's own word for itself, got:\n{answered}",
    );
}

/// The case the whole roadmap was written for: a Windows machine and the WSL on
/// it share a hostname, and the OS is what tells the two rows apart.
#[tokio::test]
async fn a_wsl_reads_linux_wsl() {
    let wsl = Reading::stated(
        Tailscale::running(vec!["verkstead-no-such-tailscale".to_owned()], 8422),
        Platform::Linux,
        Some(WSL_KERNEL.to_owned()),
        Vec::new(),
    );

    let listening = Listening::reading(THIS_DEVICE, wsl);

    let (_, answered) = asking(&listening, Showing::Nothing, peer::IDENTITY).await;

    assert_eq!(
        identity(&answered)["os"],
        "Linux (WSL)",
        "a WSL has to read as one, or a cluster holding a Windows machine and its \
         WSL would draw two rows nobody could tell apart, got:\n{answered}",
    );
}

/// And a machine with no Tailscale on it answers with its LAN addresses rather
/// than failing the answer they are part of.
#[tokio::test]
async fn a_machine_with_no_tailscale_answers_with_its_lan_alone() {
    let listening = Listening::reading(THIS_DEVICE, on_the_lan());

    let (_, answered) = asking(&listening, Showing::Nothing, peer::IDENTITY).await;

    assert_eq!(
        identity(&answered)["addresses"],
        serde_json::json!(["192.168.1.24", "10.0.0.7"]),
        "a device is reachable on what it can say it is reachable on, and a LAN \
         address is an answer, got:\n{answered}",
    );
}

/// And two callers asking in quick succession cost one `tailscale` between
/// them.
///
/// The identity endpoint is the one route nobody has to be anybody to read, on
/// a port facing the LAN, and the tailnet half of its answer is a command run on
/// this machine — so without a hold over it a stranger turns a request into a
/// process, as fast as they care to ask. The discovery stage makes that
/// ordinary rather than hostile: every device on a tailnet probing every other
/// one each time a pane is opened.
///
/// Counted by a script that writes a byte each time it runs, which is the only
/// way to ask *how many times was this run* — the answer it prints is the same
/// either way, so an assertion on the addresses would pass whether the hold
/// worked or not.
///
/// Unix only, for the reason the test below is: what stands in for `tailscale`
/// is a shell script.
#[cfg(unix)]
#[tokio::test]
async fn two_callers_in_quick_succession_cost_one_tailscale() {
    let counting = tempfile::tempdir().unwrap();
    let runs = counting.path().join("runs");

    let script = format!(
        "printf x >> {}; printf '%s' \
         '{{\"BackendState\":\"Running\",\"Self\":{{\"DNSName\":\"workbench.tailnet-name.ts.net.\",\
         \"TailscaleIPs\":[\"100.64.0.1\"]}}}}'",
        runs.display(),
    );

    let reading = Reading::stated(
        Tailscale::running(
            vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                script,
                "tailscale".to_owned(),
            ],
            8422,
        ),
        Platform::HERE,
        None,
        Vec::new(),
    );

    let listening = Listening::reading(THIS_DEVICE, reading);

    for _ in 0..3 {
        let (_, answered) = asking(&listening, Showing::Nothing, peer::IDENTITY).await;

        assert_eq!(
            identity(&answered)["addresses"],
            serde_json::json!(["workbench.tailnet-name.ts.net", "100.64.0.1"]),
            "every caller is answered the same thing, held or read, got:\n{answered}",
        );
    }

    assert_eq!(
        std::fs::read_to_string(&runs).unwrap().len(),
        1,
        "three asks in the same moment should have cost one command between them, or a \
         stranger turns a request into a process as fast as they can ask",
    );
}

/// And where Tailscale *is* up, the tailnet name and address come first: they
/// are the half a peer on another network can reach.
///
/// Unix only, for the reason the Remote access suite is: what stands in for
/// `tailscale` here is a shell script, the machine running the tests being the
/// one machine a suite about four different machines must not ask.
#[cfg(unix)]
#[tokio::test]
async fn the_tailnet_comes_before_the_lan() {
    /// A machine on a tailnet, as `tailscale status --json` describes one.
    const ON_A_TAILNET: &str = r#"printf '%s' '{"BackendState":"Running","Self":{"DNSName":"workbench.tailnet-name.ts.net.","TailscaleIPs":["100.64.0.1","fd7a:115c:a1e0::1"]}}'"#;

    let reading = Reading::stated(
        Tailscale::running(
            vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                ON_A_TAILNET.to_owned(),
                // `sh -c` gives `$0` the script's own name, so what Verkstead
                // passes lands in `$1` onwards.
                "tailscale".to_owned(),
            ],
            8422,
        ),
        Platform::HERE,
        None,
        vec![
            // The tailnet address is on an interface of its own as well, which
            // is why the list is not simply the two halves appended.
            "100.64.0.1".parse().unwrap(),
            "192.168.1.24".parse().unwrap(),
        ],
    );

    let listening = Listening::reading(THIS_DEVICE, reading);

    let (_, answered) = asking(&listening, Showing::Nothing, peer::IDENTITY).await;

    assert_eq!(
        identity(&answered)["addresses"],
        serde_json::json!([
            "workbench.tailnet-name.ts.net",
            "100.64.0.1",
            "fd7a:115c:a1e0::1",
            "192.168.1.24",
        ]),
        "the tailnet name and address lead, and the address the daemon already \
         named is not named again by the interface it is on, got:\n{answered}",
    );
}

/// Where the one route this suite stands behind the real member gate answers.
///
/// Stood up because the listener has none of its own yet: the announcement, the
/// unlink broadcast and the renewal are what put routes inside the gate, and a
/// gate with nothing behind it can only ever be asked who is *refused*. What is
/// being asked here is the other half — which callers get through — and a route
/// of one line is the whole of that question. It reads its caller back, so that
/// what came through is known to be the caller that was admitted rather than
/// somebody else on the same connection.
const MEMBERS_ONLY: &str = "/api/peer/v1/members-only";

/// A database with nothing in it, and the directory keeping it alive.
///
/// A real one rather than a stated membership, because what these tests are
/// about is a caller getting *through* the gate: a stated membership holds no
/// certificate, so it is a number for the changeover to read and can admit
/// nobody.
async fn a_store() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();

    (dir, pool)
}

/// A device written down as a member of this one's cluster, as it would be by
/// the join the stage after this builds.
///
/// A fixture rather than a link, which is the whole of what this task has: what
/// a row comes from is the join, and what the row *does* is what is being
/// asked. The fingerprint is the device's own, because that is what a caller is
/// on this listener.
async fn recorded(pool: &SqlitePool, device: &Device) {
    record_member(
        pool,
        &Linking {
            device: device.id().to_owned(),
            name: "somewhere-else".to_owned(),
            os: "Linux".to_owned(),
            addresses: vec!["192.168.1.31".to_owned()],
            fingerprint: device.fingerprint().to_owned(),
        },
    )
    .await
    .unwrap();
}

/// A listener standing on the membership `pool` keeps: everything a Verkstead
/// answers, with [`MEMBERS_ONLY`] merged in behind the real gate.
///
/// Merged rather than stood up on its own, so that the one listener can be
/// asked all three of the gate's questions — a member reaches the route, a
/// stranger does not, and a stranger still reads the identity endpoint, which
/// is outside the gate and has to stay there.
fn gated_on(id: &str, pool: SqlitePool) -> Listening {
    Listening::serving(id, move |device| {
        let members = peer::Members::recorded(pool);

        peer::router(
            device,
            nowhere(),
            members.clone(),
            peer::joining::Joins::none(),
            verkstead_server::nudge::Nudges::new(),
            Router::new(),
        )
        .merge(peer::members_only(
            Router::new().route(
                MEMBERS_ONLY,
                get(
                    |ConnectInfo(caller): ConnectInfo<peer::Caller>| async move {
                        caller.fingerprint().unwrap_or_else(|| "nothing".to_owned())
                    },
                ),
            ),
            members,
        ))
    })
}

/// The gate's other half, and the whole of what a membership is for: a caller
/// presenting a certificate this device has recorded reaches a route behind it.
///
/// The three callers together, because what the gate does is tell them apart: a
/// member gets through, a device this one has never heard of does not, and
/// neither does one showing nothing at all. And the fingerprint that comes back
/// is the member's own, so what got through is the caller that was admitted.
#[tokio::test]
async fn a_caller_presenting_a_members_certificate_reaches_a_gated_route() {
    let (_held, pool) = a_store().await;
    let listening = gated_on(THIS_DEVICE, pool.clone());
    let (member, _elsewhere) = a_stranger();

    recorded(&pool, &member).await;

    let (_, answered) = asking(&listening, Showing::A(member.clone()), MEMBERS_ONLY).await;

    assert_eq!(
        status(&answered),
        200,
        "a device this one has recorded is a member, and a member reaches what a \
         membership admits, got:\n{answered}",
    );
    assert_eq!(
        body(&answered),
        member.fingerprint(),
        "and the route reads the caller that was admitted, got:\n{answered}",
    );

    let (unknown, _somewhere) = a_device_called(A_THIRD_DEVICE);

    for (named, showing) in [
        ("a device this one has never recorded", Showing::A(unknown)),
        ("a caller showing nothing at all", Showing::Nothing),
    ] {
        let (_, answered) = asking(&listening, showing, MEMBERS_ONLY).await;

        assert_eq!(
            status(&answered),
            403,
            "{named} holds no membership here, got:\n{answered}",
        );
    }
}

/// And a member taken out of the table is refused on the next call, with
/// nothing cached from before it went.
///
/// Which is what an unlink has to mean. A membership read once and held would
/// be a device that went on being admitted for as long as the server ran, and
/// the human's press would be a row changing under a gate that had stopped
/// looking at it.
#[tokio::test]
async fn a_member_taken_out_of_the_table_is_refused_on_the_next_call() {
    let (_held, pool) = a_store().await;
    let listening = gated_on(THIS_DEVICE, pool.clone());
    let (member, _elsewhere) = a_stranger();

    recorded(&pool, &member).await;

    let (_, answered) = asking(&listening, Showing::A(member.clone()), MEMBERS_ONLY).await;
    assert_eq!(status(&answered), 200, "got:\n{answered}");

    forget_member(&pool, member.id()).await.unwrap();

    let (_, answered) = asking(&listening, Showing::A(member), MEMBERS_ONLY).await;

    assert_eq!(
        status(&answered),
        403,
        "the gate reads the table at every call, so a member that is gone is \
         gone, got:\n{answered}",
    );
}

/// And the identity endpoint is still outside the gate for all three of them,
/// which is what a device looking for somebody to link to reads.
#[tokio::test]
async fn the_identity_stays_outside_the_gate_on_a_listener_with_members() {
    let (_held, pool) = a_store().await;
    let listening = gated_on(THIS_DEVICE, pool.clone());
    let (member, _elsewhere) = a_stranger();

    recorded(&pool, &member).await;

    let (unknown, _somewhere) = a_device_called(A_THIRD_DEVICE);

    for (named, showing) in [
        ("a member", Showing::A(member)),
        ("a device this one has never recorded", Showing::A(unknown)),
        ("a caller showing nothing at all", Showing::Nothing),
    ] {
        let (_, answered) = asking(&listening, showing, peer::IDENTITY).await;

        assert_eq!(
            status(&answered),
            200,
            "{named} should read the identity endpoint: nobody has to be anybody \
             to ask a device what it is, got:\n{answered}",
        );
    }
}
