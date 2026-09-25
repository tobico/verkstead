//! The listener devices talk to each other over: TLS, every interface, a port
//! of its own (ADR-0020, *The peer listener, and mutual TLS*).
//!
//! **A second server rather than a share of the first.** The workbench listener
//! is untouched — loopback, `tailscale serve` in front of it, plain HTTP to the
//! browser, the Workbench Key gate over it — and two things ruled out putting
//! this on it. The served address carries Tailscale's certificate rather than
//! this device's, so a link pinned on a fingerprint could never go through it;
//! and the workbench port speaks plain HTTP to the browser and to the serve
//! alike. A listener that sniffed the first byte of every connection for a TLS
//! handshake was considered and rejected as a trick where a port would do. The
//! Conversation-scoped session API stays outside this entirely: it answers the
//! loopback and the named pipe, which is all a session ever dials.
//!
//! **The certificate it presents is the device's own** — see [`crate::device`],
//! which made it. That is what makes this listener worth dialling: a link
//! between two devices *is* the two fingerprints each side holds, so the
//! handshake is the identification rather than a detail of how the bytes are
//! encrypted.
//!
//! **A client certificate is asked for and not insisted on.** The request goes
//! out once per connection, before any path is known, so the handshake cannot
//! be the thing that decides which endpoints a caller reaches: it takes
//! whatever arrives — or nothing — and lets the request through to the routes.
//! A verifier that refused every non-member outright was the first shape of
//! this, and it is the shape a join could never have got through: the device
//! posting one is a stranger by definition, and the membership it is asking for
//! is what it has not got yet. What the handshake still insists on is that a
//! caller presenting a certificate holds the key that signed it — see
//! [`WhateverArrives`], which checks the signature and nothing else. The gate
//! that *acts* on the certificate is per route and comes after this.
//!
//! **The accept loop never waits on a handshake.** Each connection's is run in
//! a task of its own and the completed ones are queued, because
//! [`axum::serve::Listener::accept`] is one call at a time: a caller that
//! opened a socket and then said nothing would otherwise hold the whole peer
//! listener behind it, and this port answers a LAN that may not be the human's
//! alone.
//!
//! **And no ALPN is offered**, so every caller comes out of the handshake
//! speaking HTTP/1.1. What this listener will carry is a member's whole
//! workbench traffic — sockets for the Screen and the file watcher included —
//! and a WebSocket over HTTP/2 is a different mechanism on both sides for
//! nothing gained here.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{
    DigitallySignedStruct, DistinguishedName, ServerConfig, SignatureScheme, client::danger,
};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, UnixTime};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use verkstead_render::DeviceIdentity;

use crate::device::Device;

/// The port a device is dialled on when nobody has said otherwise, and the
/// whole of what an operator has to open on a firewall for linking to work.
///
/// One past the workbench's own, which is 8422: two ports beside each other are
/// one thing to remember, and the pair reads as an installation rather than as
/// two unrelated numbers.
pub const PEER_PORT: u16 = 8423;

/// Where a device answers for itself — the one route on this listener that asks
/// the caller for nothing at all.
///
/// Under `/api/peer/` rather than beside the workbench's `/api/v1/` and
/// `/api/ui/`, because this listener will come to serve a member the whole of
/// `/api/ui/` verbatim: a namespace of its own is what keeps a peer's own
/// business from colliding with the relayed traffic it is carried beside.
pub const IDENTITY: &str = "/api/peer/v1/identity";

/// How many completed handshakes wait to be served before the ones behind them
/// hold where they are.
///
/// A queue rather than a rendezvous because the point of it is that a slow
/// handshake blocks nobody; a bound rather than none because a caller that
/// completes handshakes and never sends a request should cost this process a
/// fixed amount of memory rather than whatever it likes.
const HANDSHAKEN: usize = 32;

/// How long an accept that failed waits before going round again.
///
/// The same second the named pipe's listener waits and axum waits, and for the
/// same reason: what fails there fails for a reason nothing in this process can
/// mend — a machine out of descriptors — and retrying it flat out is a runtime
/// spent on nothing.
const AGAIN: Duration = Duration::from_secs(1);

/// The peer listener: a socket taken, with this device's certificate standing
/// behind it.
///
/// Bound before it is served, and the two are separate calls for the reason the
/// workbench's are — an address somebody else is already listening on is a
/// startup that should fail saying so, rather than a server that came up
/// answering only half of what it promised.
pub struct Listener {
    /// The standard library's rather than the runtime's, because binding is
    /// what a start does before there is a runtime to do it on.
    socket: std::net::TcpListener,

    /// Where it really landed, which is the socket's own answer rather than
    /// what was asked for: a `:0` in a test is a port the operating system
    /// chose, and the startup line says which.
    address: SocketAddr,

    /// The certificate it presents and the verifier that takes whatever the
    /// caller shows, settled once at the bind and shared by every connection.
    presenting: Arc<ServerConfig>,
}

impl Listener {
    /// Take `address`, with `device`'s certificate to present on it.
    ///
    /// The certificate is read here rather than at the first connection, so
    /// that a device whose identity will not load is a start that stops and
    /// says so — the file it came out of is one a human can delete, and a
    /// handshake failing weeks later names nothing they could act on.
    pub fn bound(address: SocketAddr, device: &Device) -> Result<Listener> {
        let presenting = presenting(device).with_context(|| {
            format!(
                "standing the certificate in {} behind the peer listener",
                device.certificate_path().display(),
            )
        })?;

        let socket = std::net::TcpListener::bind(address)
            .with_context(|| format!("binding the peer listener to {address}"))?;

        let address = socket
            .local_addr()
            .context("asking the peer listener's socket where it landed")?;

        Ok(Listener {
            socket,
            address,
            presenting,
        })
    }

    /// Where it is listening, which is what the startup line says and what a
    /// test dials.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Serve `app` over it until the process is stopped.
    ///
    /// There is no graceful shutdown here any more than there is on the
    /// workbench listener: the process stopping is the whole of stopping, and
    /// this returning at all is a Verkstead that has stopped serving its peers.
    pub async fn serving(self, app: Router) -> Result<()> {
        self.socket
            .set_nonblocking(true)
            .context("putting the peer listener's socket into the mode the runtime reads it in")?;

        let socket = tokio::net::TcpListener::from_std(self.socket)
            .context("handing the peer listener's socket to the runtime")?;

        axum::serve(Handshaken::over(socket, self.presenting, self.address), app)
            .await
            .context("serving the peer listener")
    }
}

/// Everything this listener answers.
///
/// One route for now. The join, the dial-back that answers one and the whole of
/// a member's relayed traffic come after, and each of those is a member's or is
/// refused — this one is nobody's, which is why a device nothing has heard of
/// can read it.
pub fn router(device: Device) -> Router {
    Router::new()
        .route(IDENTITY, get(identity))
        .with_state(device)
}

/// What this device says it is.
///
/// The fingerprint travels with the id although the caller has just been handed
/// the certificate itself: what it is for is the comparison. A caller checks
/// that the device naming itself is the device that presented, and the human
/// compares the same string against what the other machine's operator is
/// reading off their own screen.
async fn identity(State(device): State<Device>) -> Json<DeviceIdentity> {
    Json(DeviceIdentity {
        device: device.id().to_owned(),
        fingerprint: device.fingerprint().to_owned(),
    })
}

/// This device's certificate and the key that signed it, as the configuration a
/// handshake is run out of.
///
/// The PEM is the file's own text, read here into the two DER halves rustls
/// takes — the identity goes straight into what will present it rather than
/// through a conversion of its own.
fn presenting(device: &Device) -> Result<Arc<ServerConfig>> {
    let pem = device.certificate().as_bytes();

    let key = PrivateKeyDer::from_pem_slice(pem).context("reading this device's private key")?;
    let certificate =
        CertificateDer::from_pem_slice(pem).context("reading this device's certificate")?;

    // `ring`, named rather than taken from rustls' own default, which is
    // `aws-lc-rs` and would put a second cryptographic library and a C
    // toolchain into a build that already has this one under reqwest. Named
    // here rather than installed process-wide for the same reason the validity
    // is written down in `device`: what this server runs on should be readable
    // where it is decided.
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let algorithms = provider.signature_verification_algorithms;

    let presenting = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("settling the protocol versions the peer listener speaks")?
        .with_client_cert_verifier(Arc::new(WhateverArrives { algorithms }))
        .with_single_cert(vec![certificate], key)
        .context("presenting this device's certificate")?;

    Ok(Arc::new(presenting))
}

/// The verifier that asks every caller for a certificate and accepts whichever
/// one turns up, or none.
///
/// **It decides nothing about who may do what.** That is the whole point: the
/// request goes out before a path is known, so a verifier refusing here would
/// be refusing a caller for endpoints it was never reaching for. What it does
/// do is make the certificate mean something — the signature is checked, so a
/// caller presenting one has the key that signed it rather than a copy of
/// somebody else's file — and hand it on. The member list is consulted per
/// route, over the routes a membership is what admits.
///
/// Nothing is checked against a trust anchor because there is no such thing
/// here: every certificate in a cluster is self-signed and pinned by
/// fingerprint, so a chain to a root would be a question with no answer.
#[derive(Debug)]
struct WhateverArrives {
    /// The signature algorithms the provider above supports, which is what the
    /// two signature checks below are run against.
    algorithms: WebPkiSupportedAlgorithms,
}

impl ClientCertVerifier for WhateverArrives {
    /// Asked for, at every connection.
    fn offer_client_auth(&self) -> bool {
        true
    }

    /// And not insisted on: a caller with nothing to show completes the
    /// handshake and reaches the un-gated surface.
    fn client_auth_mandatory(&self) -> bool {
        false
    }

    /// No hints, because there are no trust anchors to hint at. An empty list
    /// is also what tells a caller to send whatever certificate it has rather
    /// than looking for one issued by somebody named here.
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    /// Whatever arrived, taken. Not parsed and not judged: what a certificate
    /// on this listener means is *which* device it is, and that is a
    /// fingerprint compared against a member list rather than anything a
    /// verifier could work out.
    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        Ok(ClientCertVerified::assertion())
    }

    /// The one thing that *is* checked, on a TLS 1.2 handshake: that the caller
    /// holds the private key for the certificate it presented. Without this a
    /// certificate would be a public file anybody could replay, and the member
    /// list it will be checked against would admit whoever had read one.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<danger::HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    /// And the same on a TLS 1.3 one, which is what every device in a cluster
    /// of this build actually negotiates.
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<danger::HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

/// The socket with the handshakes already done, which is what axum is handed.
///
/// [`axum::serve::Listener::accept`] is called once at a time and the
/// connection it hands back is served the moment it returns, so a handshake run
/// inside it would be a handshake every other caller waits behind. Here the
/// accepting is a task of its own, each handshake is a task under it, and this
/// is the near end of the queue they finish into.
struct Handshaken {
    address: SocketAddr,
    completed: mpsc::Receiver<(TlsStream<TcpStream>, SocketAddr)>,
}

impl Handshaken {
    /// Start accepting on `socket`, securing what arrives with `presenting`.
    fn over(
        socket: tokio::net::TcpListener,
        presenting: Arc<ServerConfig>,
        address: SocketAddr,
    ) -> Handshaken {
        let (done, completed) = mpsc::channel(HANDSHAKEN);

        tokio::spawn(accepting(socket, TlsAcceptor::from(presenting), done));

        Handshaken { address, completed }
    }
}

impl axum::serve::Listener for Handshaken {
    type Io = TlsStream<TcpStream>;

    /// Where the caller dialled from. What a route will want to know about a
    /// peer is which device it is, and that is the certificate rather than the
    /// address — a laptop's changes between the LAN and the tailnet, which is
    /// why a device advertises all of them.
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.completed.recv().await {
            Some(connection) => connection,

            // The accepting task holds the other end for as long as it runs,
            // and it only ever returns when this end has gone — so this is
            // unreachable while anything is being served. Waiting for ever
            // rather than panicking, because the alternative to an answer here
            // is not a better answer: a listener with nothing behind it has no
            // connection to hand over, and taking the process down with it
            // would stop the workbench too.
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(self.address)
    }
}

/// Accept for ever, handing each connection its own handshake.
///
/// Nothing here waits on one. A caller that opens a socket and says nothing
/// holds its own task and nobody else's, and what it costs this process is one
/// pending handshake until the connection is dropped at the other end.
async fn accepting(
    socket: tokio::net::TcpListener,
    securing: TlsAcceptor,
    done: mpsc::Sender<(TlsStream<TcpStream>, SocketAddr)>,
) {
    loop {
        let (connection, from) = match socket.accept().await {
            Ok(accepted) => accepted,
            Err(what) => {
                // The accept error axum's own listener describes: said, waited
                // out, and gone round again rather than an end to the listener.
                tracing::warn!(%what, "the peer listener could not accept a connection");
                tokio::time::sleep(AGAIN).await;
                continue;
            }
        };

        let securing = securing.clone();
        let done = done.clone();

        tokio::spawn(async move {
            match securing.accept(connection).await {
                // A full queue holds this task rather than dropping the
                // connection: the caller has completed a handshake and is
                // waiting to be answered, and the wait is the backpressure.
                Ok(secured) => {
                    let _ = done.send((secured, from)).await;
                }

                // And a handshake that did not complete is nothing to report
                // above debug. It is a port scan, a browser that would not
                // trust a self-signed certificate, or a device whose clock is
                // wrong — none of them this server's to do anything about, and
                // all of them things a reachable port sees.
                Err(what) => {
                    tracing::debug!(%from, %what, "a caller did not complete the peer handshake");
                }
            }
        });
    }
}
