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
//! is what neither of them has: what the identity endpoint answers them, every
//! other path on this listener refuses them, and the refusal is read for what
//! it says as much as for its status.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::ConnectInfo;
use axum::routing::get;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use verkstead_server::device::reading::Reading;
use verkstead_server::device::{Device, RENEW_WITHIN};
use verkstead_server::peer;
use verkstead_server::platform::{self, Platform};
use verkstead_server::remote::Tailscale;

/// The id this device is stated as, so that what a test asserts against is a
/// string it chose rather than sixteen random bytes it has to filter out of a
/// payload — see [`Device::stated`], which is here for that reason.
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the one the stranger below is stated as. A device this one has never
/// been linked to, which in this stage is every device there is.
const SOME_OTHER_DEVICE: &str = "0011223344556677889900aabbccddee";

/// The join post, which is the stage after this one's. Asked for here because
/// what this stage claims about it is that it is refused along with everything
/// else — a caller reaching for it meets the gate rather than a missing path.
const THE_JOIN_TO_COME: &str = "/api/peer/v1/join";

/// And a path no stage will ever answer, which on this listener is refused the
/// same way: a caller with no membership has no business being told which of
/// this device's endpoints exist.
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
            peer::router(device, reading, peer::Members::none())
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
    fn mid_changeover(id: &str) -> Listening {
        let dir = tempfile::tempdir().unwrap();

        Device::stated_good_for(dir.path(), id, RENEW_WITHIN - time::Duration::days(1)).unwrap();

        let device = Device::issued(dir.path(), &peer::Members::stated(1)).unwrap();

        assert!(
            device.incoming_fingerprint().is_some(),
            "a start inside the renewal window makes a fresh certificate",
        );

        Listening::standing(dir, device, |device| {
            peer::router(device, nowhere(), peer::Members::stated(1))
        })
    }

    /// The socket, the handshakes and the serve, over a device that is already
    /// made: what every constructor above comes down to once it has said which
    /// device it is standing behind.
    fn standing(
        dir: tempfile::TempDir,
        device: Device,
        answering: impl FnOnce(Device) -> Router,
    ) -> Listening {
        let listener = peer::Listener::bound("127.0.0.1:0".parse().unwrap(), &device)
            .expect("the loopback on a port the machine picked is free");
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

/// Dial `listening`, ask for `path`, and hand back what came of it: the
/// certificate the server presented, and the response as it arrived.
async fn asking(
    listening: &Listening,
    showing: Showing,
    path: &str,
) -> (CertificateDer<'static>, String) {
    // Whatever the server shows is taken, the way a device linking for the
    // first time takes it: what proves the far end is the certificate compared
    // against a fingerprint afterwards, and there is no certificate authority
    // anywhere in a cluster to check one against.
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

    // Dialled by the name the certificate is made out to, which is the device
    // id — that is what a peer will have to hand, and the only name this
    // certificate has.
    let name = ServerName::try_from(listening.device.id().to_owned()).unwrap();

    let connection = TcpStream::connect(listening.address).await.unwrap();
    let mut secured = TlsConnector::from(Arc::new(dialling))
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
    let listening = Listening::mid_changeover(THIS_DEVICE);

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

/// A device this one has never heard of, standing where a stranger stands.
///
/// The directory comes back with it because the identity lives in it: dropped
/// early, the certificate goes out from under the handle still presenting it.
fn a_stranger() -> (Device, tempfile::TempDir) {
    let elsewhere = tempfile::tempdir().unwrap();
    let stranger = Device::stated(elsewhere.path(), SOME_OTHER_DEVICE).unwrap();

    (stranger, elsewhere)
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
/// reaches the identity endpoint — which the suite above proves — and nothing
/// else on the listener.
#[tokio::test]
async fn a_caller_with_no_certificate_reaches_nothing_but_the_identity() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);

    for path in [THE_JOIN_TO_COME, NOTHING_ANSWERS_THIS] {
        let (_, answered) = asking(&listening, Showing::Nothing, path).await;

        assert_eq!(
            status(&answered),
            403,
            "{path} is a member's or is refused, and there is no member, got:\n{answered}",
        );
    }
}

/// And the second: a certificate nothing here has recorded completes the
/// handshake — the suite above proves that too — and is refused by every route
/// the gate stands over.
#[tokio::test]
async fn a_caller_showing_an_unknown_certificate_is_refused_by_the_gated_routes() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);
    let (stranger, _elsewhere) = a_stranger();

    for path in [THE_JOIN_TO_COME, NOTHING_ANSWERS_THIS] {
        let (_, answered) = asking(&listening, Showing::A(stranger.clone()), path).await;

        assert_eq!(
            status(&answered),
            403,
            "a certificate is not a membership — the gate is what holds one, got:\n{answered}",
        );
    }
}

/// What the refusal says, which is the thing the stage after this one reads it
/// for.
///
/// A device posting a join has two ways of not getting through: a Verkstead
/// that will not have it, and a Verkstead too old to have the route at all. It
/// wants a human to press Allow in the first case and an upgrade on the other
/// machine in the second, so a refusal that read as a missing path would leave
/// it unable to say which it had met.
#[tokio::test]
async fn the_refusal_says_it_is_a_membership_rather_than_a_missing_path() {
    let listening = Listening::with_the_device_called(THIS_DEVICE);
    let (stranger, _elsewhere) = a_stranger();

    let (_, answered) = asking(&listening, Showing::A(stranger), THE_JOIN_TO_COME).await;

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
