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
//! is what it has not got yet. What the handshake still insists on is that the
//! certificate is a certificate: that the caller holds the key that signed it,
//! and that it is inside its own validity — see [`WhateverArrives`], which
//! asks those two and nothing else. The second is where an expired certificate
//! is refused, which is the whole of what the renewal in [`crate::device`] is
//! for: nothing in a cluster checks a chain and a member list of fingerprints
//! would go on matching one for ever, so this is the only place the expiry is a
//! fact. What the certificate *means* — which device this is, and whether it is
//! one of ours — is a per-route question, and [`gate`] is where it is asked.
//!
//! **The un-gated surface is a list of three, and everything else is a
//! member's.** The identity endpoint, which asks for no certificate at all;
//! the join post, which comes from a non-member by definition and whose
//! certificate is pinned into the pending request it creates; and the dial-back
//! answering a join, matched against the certificate that pending request is
//! holding. The first two are here — see [`joining`], which carries the post and
//! the cancel that takes a question back, both of them matched against that same
//! pinned certificate — and the dial-back is still to come. [`gate`] stands over
//! everything else — asking [`Members`] of every caller,
//! which is rows now rather than a number nobody wrote. The refusal says what
//! it is: a caller this device holds no membership for, rather than a path that
//! is not there. A device posting a join has to be able to tell a Verkstead that
//! will not have it from one too old to have the route at all.
//!
//! **The accept loop never waits on a handshake, and no handshake waits for
//! ever.** Each connection's is run in a task of its own and the completed ones
//! are queued, because [`axum::serve::Listener::accept`] is one call at a time:
//! a caller that opened a socket and then said nothing would otherwise hold the
//! whole peer listener behind it, and this port answers a LAN that may not be
//! the human's alone. And each of those tasks is given [`HANDSHAKE`] and no
//! longer, because the same caller holds a task and a descriptor while it says
//! nothing — with no deadline, how long that lasts is the caller's to decide,
//! and a host on the LAN can open sockets until this process is out of
//! descriptors and the workbench has gone with it.
//!
//! **And no ALPN is offered**, so every caller comes out of the handshake
//! speaking HTTP/1.1. What this listener will carry is a member's whole
//! workbench traffic — sockets for the Screen and the file watcher included —
//! and a WebSocket over HTTP/2 is a different mechanism on both sides for
//! nothing gained here.
//!
//! **And the outbound half is [`dialling`]**, which is how this device reaches
//! one of these rather than answers on one. Everything above is what a caller
//! meets; that is what this end *is* when it is the caller — the certificate
//! presented, the far end's checked against the fingerprint a member row holds,
//! and every address that row carries tried in the order it was advertised.

pub mod dialling;
pub mod joining;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::Json;
use axum::Router;
use axum::extract::connect_info::{ConnectInfo, Connected};
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::serve::IncomingStream;
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{
    DigitallySignedStruct, DistinguishedName, ServerConfig, SignatureScheme, client::danger,
};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, UnixTime};
use sqlx::SqlitePool;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use verkstead_render::{DeviceIdentity, LinkedDevice};
use verkstead_store::Member;
use x509_parser::certificate::X509Certificate;
use x509_parser::prelude::FromDer;

use crate::device::Device;
use crate::device::reading::Reading;
use joining::Joins;

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

/// How long a caller has to get through the handshake before the connection is
/// let go of.
///
/// **Because a connection nobody finishes costs this process something until
/// somebody does.** Each handshake is a task and a descriptor, and which of
/// them ever completes is the caller's to decide — so without a deadline a host
/// on the LAN opens sockets, says nothing, and holds a task and a descriptor
/// each for as long as it likes. Running out of descriptors takes the workbench
/// down with this listener, the two being served out of one process.
///
/// Ten seconds, which is a handshake over a link bad enough that the call after
/// it would not have worked either, and nowhere near long enough to be worth
/// anybody's while to sit in. A caller let go of here is free to dial again,
/// which is what a device on a slow link does.
const HANDSHAKE: Duration = Duration::from_secs(10);

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

    /// How long a caller has to get through the handshake — [`HANDSHAKE`] on a
    /// running server, and whatever a suite says when it is standing where a
    /// caller has gone quiet.
    handshake: Duration,
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
            handshake: HANDSHAKE,
        })
    }

    /// The same listener, giving a caller `handshake` to get through the
    /// handshake in rather than [`HANDSHAKE`].
    ///
    /// For a suite standing where a caller has opened a socket and gone quiet:
    /// what is being asked is that the connection is let go of at all, and ten
    /// seconds of a test waiting to watch it happen would be ten seconds spent
    /// on the clock rather than on the question.
    pub fn handshaking_within(self, handshake: Duration) -> Listener {
        Listener { handshake, ..self }
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

        axum::serve(
            Handshaken::over(socket, self.presenting, self.address, self.handshake),
            // With a [`Caller`] beside every request, which is how the
            // certificate the handshake took gets out of the connection and
            // into a route. It is a fact about the connection rather than
            // about any one request carried on it, so it is put there by the
            // serve rather than by anything a caller sends — there is no
            // header a peer could write to say who it is.
            app.into_make_service_with_connect_info::<Caller>(),
        )
        .await
        .context("serving the peer listener")
    }
}

/// Everything this listener answers: the un-gated surface, with everything
/// else behind [`gate`].
///
/// Three routes stand outside it. The identity endpoint, which is nobody's,
/// which is why a device nothing has heard of can read it; the join post, which
/// comes from a device this one holds no membership for, that being what a join
/// is; and the cancel that takes a join back, which comes from the same device
/// and is matched against the same pinned certificate — see [`joining`]. The
/// dial-back that answers an Allow comes out here beside them in the task that
/// adds it. A member's relayed traffic goes the other way, inside
/// [`members_only`] with the gate over it.
///
/// **What is not a route is refused rather than missed**, because the gate is
/// the fallback: a path nothing here answers is one this caller has no
/// business asking after either, and a listener that said which of its
/// endpoints existed would be telling a stranger what to reach for. That is
/// the Workbench Key's own arrangement, where a path under `/api/` that no
/// route answers is refused at the gate rather than missed at the fallback.
pub fn router(device: Device, reading: Reading, members: Members, joins: Joins) -> Router {
    Router::new()
        .route(IDENTITY, get(identity))
        .with_state(Answering {
            device: device.clone(),
            reading: reading.clone(),
        })
        .merge(
            Router::new()
                .route(joining::JOIN, post(joining::join))
                .route(joining::CANCEL, post(joining::cancel))
                .with_state(joining::Holding {
                    device,
                    reading,
                    joins,
                }),
        )
        .fallback_service(members_only(Router::new(), members))
}

/// What the identity endpoint answers out of: what this device *is*, and the
/// machine it is on.
///
/// Two handles rather than one, because they are two different kinds of thing
/// kept two different ways. The id and the certificate are read off the disk
/// once at the start and do not change under a running server; the name, the OS
/// and the addresses are read at the moment somebody asks and are kept nowhere
/// — see [`crate::device::reading`].
#[derive(Debug, Clone)]
struct Answering {
    device: Device,
    reading: Reading,
}

/// Everything a membership admits, with the gate over the lot of it.
///
/// `routes` rather than a router built in here, because the gate is one layer
/// over everything a member may reach and axum applies a layer to the routes
/// added *before* it: a member-only route added after the fact would be a route
/// outside the gate, and a route added without its check is the kind of mistake
/// that reads as working code. So there is one call, and everything that goes
/// through it is gated.
///
/// Empty on this listener until the stages that need routes arrive — the
/// announcement, the unlink broadcast and the renewal. It is public all the
/// same, because the suite stands its own one-line route behind the real gate:
/// what is being asked of the gate is which callers get *through* it, and a
/// gate with nothing behind it can only ever be asked who is refused.
pub fn members_only(routes: Router, members: Members) -> Router {
    routes.layer(axum::middleware::from_fn_with_state(members, gate))
}

/// The devices this one is linked to, as the things that ask after them do: the
/// gate over every route a membership admits, the changeover a re-issued
/// certificate is in the middle of, the Devices section of the Remote access
/// pane — and a dial, which is the one of them that writes, recording what it
/// found of a member at the far end of one.
///
/// **Rows, read at the moment each question is asked.** A join writes one and
/// an unlink takes one away — see [`verkstead_store::Member`] — and nothing
/// here holds a copy between two questions: a membership that was cached would
/// be a device that went on being admitted after the human unlinked it, which
/// is the one thing an unlink has to mean.
///
/// A handle over the store rather than the pool itself, so that each caller
/// asks a membership its own question rather than writing its own query — and
/// so that a suite can state one.
#[derive(Debug, Clone)]
pub struct Members {
    recorded: Recorded,
}

/// Where a membership's answers come from.
#[derive(Debug, Clone)]
enum Recorded {
    /// The rows this Verkstead keeps, read afresh at every question. Which is
    /// every running server.
    InTheStore(SqlitePool),

    /// And the membership a fixture states: this many devices, none of which
    /// this end holds a certificate for.
    ///
    /// Here for the reason [`Device::stated`] is. What a changeover does
    /// depends on whether anybody is owed an announcement of the new
    /// fingerprint, and a suite about the renewal stands on nothing but that —
    /// so it says how many there are rather than standing a store up to hold
    /// them. Nothing is admitted by one: a stated membership holds no
    /// fingerprint, so [`Members::holds`] is false for every caller, and a test
    /// that wants a caller through the gate records a real row.
    Stated(usize),
}

impl Members {
    /// None of them, which is what a Verkstead that has never been linked to
    /// anything has.
    pub fn none() -> Members {
        Members {
            recorded: Recorded::Stated(0),
        }
    }

    /// And the membership a fixture states: `linked` devices, none of which has
    /// acknowledged anything — see [`Recorded::Stated`].
    pub fn stated(linked: usize) -> Members {
        Members {
            recorded: Recorded::Stated(linked),
        }
    }

    /// The membership this Verkstead really keeps: the rows in `pool`.
    ///
    /// Built at the start, after the database is open and before the identity
    /// is read — the identity consults it, which is why the two are in that
    /// order. See `run_on_keyed`.
    pub fn recorded(pool: SqlitePool) -> Members {
        Members {
            recorded: Recorded::InTheStore(pool),
        }
    }

    /// Whether the device whose certificate has this fingerprint is one of
    /// them.
    ///
    /// The fingerprint rather than the device id, because the id is a string
    /// in a payload and anybody may write one: what a member *is* on this
    /// listener is the certificate it presented at the handshake, and the
    /// fingerprint is that certificate said in the spelling both ends of a
    /// link compare — see [`Device::fingerprint`].
    ///
    /// **A membership that cannot be read admits nobody.** This is the gate's
    /// own question, and the two ways of not knowing the answer are not the
    /// same: refusing a member while the database is unreachable costs a call
    /// that is retried, and admitting a stranger because the read failed costs
    /// the whole of what the gate is for. So a failure is said in the log and
    /// answered no.
    async fn holds(&self, fingerprint: &str) -> bool {
        match &self.recorded {
            Recorded::InTheStore(pool) => {
                match verkstead_store::member_holding(pool, fingerprint).await {
                    Ok(held) => held,
                    Err(what) => {
                        tracing::error!(
                            %what,
                            "this device's membership could not be read, so the caller \
                             presenting a certificate is refused rather than admitted",
                        );

                        false
                    }
                }
            }

            // A stated membership holds no certificate, so it admits nobody.
            Recorded::Stated(_) => false,
        }
    }

    /// How many of them have yet to acknowledge `fingerprint`, which is the
    /// question a changeover asks — see [`crate::device::Changeover`].
    ///
    /// **All of them, whatever the fingerprint is**, and now read off the rows
    /// rather than off a number nobody wrote. An acknowledgement is a member
    /// answering an announcement, the announcement is this stage's last task,
    /// and there is nowhere yet to record one — so every member is owed, and a
    /// membership with nobody in it owes nought. Nought is what completes a
    /// changeover at the start that began it, which is what a Verkstead that
    /// has never been linked to anything still does.
    ///
    /// What the task that adds the announcement fills in is the other half:
    /// this becomes the members that have not yet said they hold the new
    /// fingerprint, and each acknowledgement takes one off the list.
    pub(crate) async fn unacknowledged(&self, _fingerprint: &str) -> Result<usize> {
        match &self.recorded {
            Recorded::InTheStore(pool) => verkstead_store::member_count(pool)
                .await
                .context("asking how many members are owed an announcement"),

            Recorded::Stated(linked) => Ok(*linked),
        }
    }

    /// And every one of them as it answers for itself, which is what the
    /// Devices section of the Remote access pane draws a row from — see
    /// [`crate::device::Devices`].
    ///
    /// The same shape a device answers a stranger with on [`IDENTITY`],
    /// because it is the same thing said: what was read off the far end's
    /// machine at the last exchange, kept because this end cannot read another
    /// machine's hostname for itself.
    ///
    /// A stated membership draws nothing. It is a number and not a set of
    /// devices, and a row invented to make the count come out would be a row
    /// naming a machine that does not exist.
    pub(crate) async fn listed(&self) -> Result<Vec<LinkedDevice>> {
        Ok(self
            .rows()
            .await?
            .into_iter()
            .map(|member| LinkedDevice {
                reachable: member.reachable,
                identity: DeviceIdentity {
                    device: member.device,
                    fingerprint: member.fingerprint,
                    name: member.name,
                    os: member.os,
                    addresses: member.addresses,
                },
            })
            .collect())
    }

    /// Every member as the table holds one: the identity above, and the two
    /// things that are this device's own findings rather than the far end's —
    /// when it was last heard from, and whether the last dial got through.
    ///
    /// What [`Members::listed`] is drawn from, and what a dial is handed one of:
    /// the addresses and the fingerprint are the whole of how a member is
    /// reached — the list in the order it was advertised, and the certificate
    /// the far end has to turn out to be presenting. See
    /// [`dialling::Peers::identity`].
    pub(crate) async fn rows(&self) -> Result<Vec<Member>> {
        match &self.recorded {
            Recorded::InTheStore(pool) => verkstead_store::members(pool)
                .await
                .context("reading the devices this one is linked to"),

            Recorded::Stated(_) => Ok(Vec::new()),
        }
    }

    /// Write down what a member said about itself at an exchange that has just
    /// got through — see [`verkstead_store::record_member`], which is where the
    /// moment and the un-dimming come from.
    ///
    /// One of the two things here that write, the mark below being the other. A
    /// membership is read at every question so that an unlink means something,
    /// and this is the other half of that: a row is what a dial found rather
    /// than what a start remembered, so a device that moved or was renamed is
    /// right again on the next call to it.
    ///
    /// A stated membership has no rows to write, so nothing is written. It is a
    /// number, and a suite standing on one is asking about a changeover rather
    /// than about a member.
    pub(crate) async fn refreshed(&self, linking: &verkstead_store::Linking) -> Result<()> {
        match &self.recorded {
            Recorded::InTheStore(pool) => verkstead_store::record_member(pool, linking)
                .await
                .with_context(|| format!("recording what device {} says it is", linking.device)),

            Recorded::Stated(_) => Ok(()),
        }
    }

    /// And mark one as answering nothing, which is what a dial that reached none
    /// of its addresses does — see [`verkstead_store::member_unreachable`].
    ///
    /// Nothing else about the row moves: it stays on the list, dimmed, with its
    /// addresses and its fingerprint exactly as they were, and the next dial
    /// that gets through puts it back.
    pub(crate) async fn unreachable(&self, device: &str) -> Result<()> {
        match &self.recorded {
            Recorded::InTheStore(pool) => verkstead_store::member_unreachable(pool, device)
                .await
                .with_context(|| format!("marking device {device} as answering nothing")),

            Recorded::Stated(_) => Ok(()),
        }
    }
}

/// The member gate: everything [`members_only`] carries is a member's or is
/// refused.
///
/// The certificate was taken at the handshake, before any path was known — see
/// [`WhateverArrives`] — so this is the first place a path and a caller are
/// known together, and it is the only place either is judged.
async fn gate(
    State(members): State<Members>,
    ConnectInfo(caller): ConnectInfo<Caller>,
    request: Request,
    next: Next,
) -> Response {
    match caller.fingerprint() {
        Some(fingerprint) if members.holds(&fingerprint).await => next.run(request).await,
        _ => refused(),
    }
}

/// And the refusal, which is one answer for a caller that showed nothing and a
/// caller that showed a certificate nothing here has recorded: both are a
/// device this one holds no membership for, and there is nothing else to say
/// to either.
///
/// **`Forbidden` rather than `Not Found`**, and it says which it is. A device
/// posting a join has two ways of not getting through — a Verkstead that will
/// not have it, and a Verkstead too old to have the route at all — and those
/// want different things of it: the first is a human to press Allow, the
/// second an upgrade on the other machine. A refusal that read as a missing
/// path would leave the two indistinguishable.
///
/// No `WWW-Authenticate`, for the reason the Workbench Key's refusal carries
/// none — see [`crate::key`]. The credential here is a certificate, and it was
/// asked for at the handshake and already given or already withheld: a
/// challenge after the fact is a question this caller cannot answer on this
/// connection.
fn refused() -> Response {
    (
        StatusCode::FORBIDDEN,
        "you are not a member of this verkstead's cluster\n",
    )
        .into_response()
}

/// Who is on the far end of a connection: where they dialled from, and the
/// certificate the handshake took from them, where they showed one.
///
/// Read off the connection rather than off the request, and put beside every
/// request on it by [`Listener::serving`]. What a route wants to know about a
/// peer is which device it is, and on this listener that is the certificate —
/// the address is a laptop's and moves between the LAN and the tailnet, which
/// is why a device advertises all of them.
#[derive(Debug, Clone)]
pub struct Caller {
    /// Where the connection came from. Not what says who this is, and kept for
    /// the things an address is good for: a dial-back to a device that has just
    /// asked to join, and a line in a log.
    dialled_from: SocketAddr,

    /// And what they presented, where they presented anything. `None` is an
    /// ordinary caller rather than a failure: the handshake asks and does not
    /// insist, and a device asking *who are you* has nothing to show yet — it
    /// is asking because it does not know.
    presented: Option<CertificateDer<'static>>,
}

impl Caller {
    /// Where the connection came from.
    pub fn dialled_from(&self) -> SocketAddr {
        self.dialled_from
    }

    /// The certificate itself, which is what a pending join pins: a
    /// fingerprint says whether two certificates are the same one, and the
    /// dial-back that answers a join has to check the far end against the
    /// certificate rather than against a string about it.
    pub fn presented(&self) -> Option<&CertificateDer<'static>> {
        self.presented.as_ref()
    }

    /// And its fingerprint, in the one spelling this tree prints one in — see
    /// [`Device::fingerprint`]. That is what a member list is keyed by and what
    /// two people compare by eye, so the caller's is said the same way or the
    /// comparison is between two strings about the same certificate.
    pub fn fingerprint(&self) -> Option<String> {
        self.presented
            .as_ref()
            .map(crate::device::fingerprint_of_der)
    }
}

impl Connected<IncomingStream<'_, Handshaken>> for Caller {
    /// Taken off the finished handshake, which is the only place it is: rustls
    /// hands the peer's chain over once the connection is secured, and the end
    /// entity — the first of it — is the device.
    ///
    /// Nothing in the chain past that is looked at. Every certificate in a
    /// cluster is self-signed and pinned by fingerprint, so an intermediate
    /// would be something a caller had appended rather than something a
    /// membership could ever have recorded.
    fn connect_info(stream: IncomingStream<'_, Handshaken>) -> Caller {
        let presented = stream
            .io()
            .get_ref()
            .1
            .peer_certificates()
            .and_then(<[CertificateDer<'_>]>::first)
            .map(|certificate| certificate.clone().into_owned());

        Caller {
            dialled_from: *stream.remote_addr(),
            presented,
        }
    }
}

/// What this device says it is.
///
/// The fingerprint travels with the id although the caller has just been handed
/// the certificate itself: what it is for is the comparison. A caller checks
/// that the device naming itself is the device that presented, and the human
/// compares the same string against what the other machine's operator is
/// reading off their own screen.
///
/// The name, the OS and the addresses beside them are read off the machine as
/// this is answered rather than held from the start — see
/// [`crate::device::reading`]. Which is why the handler is `async` for a route
/// that has nothing to wait on otherwise: the tailnet half of the addresses is
/// a command run on the machine, and a peer that asked a moment later would get
/// a different and equally true answer.
async fn identity(State(answering): State<Answering>) -> Json<DeviceIdentity> {
    Json(answering.reading.identity(&answering.device).await)
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
/// request goes out before a path is known, so a verifier refusing on *who*
/// this is would be refusing a caller for endpoints it was never reaching for.
/// The member list is consulted per route, over the routes a membership is what
/// admits.
///
/// **What it does insist on is that the certificate is one at all**, and there
/// are two halves to that. The signature is checked, so a caller presenting a
/// certificate has the key that signed it rather than a copy of somebody else's
/// file. And the validity dates are checked against the clock rustls hands in,
/// so a certificate that has run out — or has not started — is refused here.
///
/// That second half is what the whole renewal rests on. A device re-issues its
/// certificate before it expires because an expired one is refused at the
/// handshake; nothing in a cluster checks a chain, so unless it is checked
/// *here* nothing checks it anywhere, and the validity, the renewal window and
/// the changeover would be machinery holding nothing up. Refusing it is not
/// deciding who may do what — it is the same class of question as the signature,
/// asked of the certificate rather than of the device behind it.
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

    /// Whatever arrived, taken — so long as it is a certificate and is inside
    /// its own validity.
    ///
    /// *Which* device it is is not asked here: that is a fingerprint compared
    /// against a member list, and a path this verifier has not been told. What
    /// *is* asked is the one question a certificate answers about itself, and
    /// the answer to it is what the renewal exists for — see
    /// [`current`] and [`crate::device::VALIDITY`].
    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        current(end_entity, now)?;

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

/// Whether `certificate` is inside its own validity at `now`, which is the
/// clock rustls read as the handshake began.
///
/// **The one thing the renewal rests on.** A device makes its certificate again
/// before the ninety days are up because an expired one is refused at the
/// handshake — and in a cluster nothing checks a chain, nothing asks an
/// authority, and the member list is a set of fingerprints that goes on
/// matching a certificate for ever. So this is the only place the expiry is a
/// fact rather than a date printed in a file, and without it the validity, the
/// renewal window and the changeover would all be holding nothing up.
///
/// A certificate that will not parse is refused here too. It reached this
/// verifier as a client certificate and is not one, which is a different thing
/// from the caller that showed nothing at all — that one never comes through
/// here.
///
/// Said as rustls' own two reasons rather than as one, because they are two
/// different machines to go and look at: an expired certificate is one whose
/// Verkstead has not been restarted since the renewal window opened, and one
/// that has not started yet is a clock that is wrong.
fn current(certificate: &CertificateDer<'_>, now: UnixTime) -> Result<(), rustls::Error> {
    let (_, parsed) = X509Certificate::from_der(certificate)
        .map_err(|_| rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding))?;

    let validity = parsed.validity();
    let now = now.as_secs() as i64;

    if now < validity.not_before.timestamp() {
        return Err(rustls::Error::InvalidCertificate(
            rustls::CertificateError::NotValidYet,
        ));
    }

    if now > validity.not_after.timestamp() {
        return Err(rustls::Error::InvalidCertificate(
            rustls::CertificateError::Expired,
        ));
    }

    Ok(())
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
    /// Start accepting on `socket`, securing what arrives with `presenting`
    /// and giving each caller `handshake` to get through it in.
    fn over(
        socket: tokio::net::TcpListener,
        presenting: Arc<ServerConfig>,
        address: SocketAddr,
        handshake: Duration,
    ) -> Handshaken {
        let (done, completed) = mpsc::channel(HANDSHAKEN);

        tokio::spawn(accepting(
            socket,
            TlsAcceptor::from(presenting),
            done,
            handshake,
        ));

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
/// holds its own task and nobody else's — and holds it for [`HANDSHAKE`] and
/// not a moment longer, because how long that is is this end's to decide rather
/// than the caller's: a task and a descriptor apiece, handed out to whoever asks
/// and given back when they feel like it, is a port anybody on the LAN can run
/// this process out of descriptors through.
async fn accepting(
    socket: tokio::net::TcpListener,
    securing: TlsAcceptor,
    done: mpsc::Sender<(TlsStream<TcpStream>, SocketAddr)>,
    handshake: Duration,
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
            match tokio::time::timeout(handshake, securing.accept(connection)).await {
                // A full queue holds this task rather than dropping the
                // connection: the caller has completed a handshake and is
                // waiting to be answered, and the wait is the backpressure.
                Ok(Ok(secured)) => {
                    let _ = done.send((secured, from)).await;
                }

                // And a handshake that did not complete is nothing to report
                // above debug. It is a port scan, a browser that would not
                // trust a self-signed certificate, or a device whose clock is
                // wrong — none of them this server's to do anything about, and
                // all of them things a reachable port sees.
                Ok(Err(what)) => {
                    tracing::debug!(%from, %what, "a caller did not complete the peer handshake");
                }

                // And one that ran out of time is the same thing said a
                // different way: the connection is dropped here, which is what
                // closes the socket and gives the descriptor back. A device on
                // a link this slow dials again, and a caller that never meant
                // to finish has cost this process [`HANDSHAKE`] and nothing
                // more.
                Err(_) => {
                    tracing::debug!(
                        %from,
                        "a caller opened a connection and did not get through the peer \
                         handshake in time",
                    );
                }
            }
        });
    }
}
