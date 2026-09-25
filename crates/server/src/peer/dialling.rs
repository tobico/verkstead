//! The outbound half of the peer listener: how this device reaches another one
//! (ADR-0020, *The peer listener, and mutual TLS*, and *Addresses*).
//!
//! [`super`] is what a caller meets. This is what this end *is* when it is the
//! caller — and it is the one way anything in this tree dials a device, because
//! everything a cluster does past being reachable goes through here: the join,
//! the dial-back that answers one, the announcement of a newcomer, the unlink
//! broadcast and the announcement of a renewed certificate.
//!
//! **The certificate presented is this device's own**, which over a
//! [`Changeover`](crate::device::Changeover) is the outgoing one — see
//! [`Device::certificate`]. What a member holds is what a member has to be
//! answered with, and that is as true of a call this device makes as of one it
//! takes: a dial presenting a certificate the far end has never acknowledged
//! would be refused at that end's own gate.
//!
//! **And the far end is proved by the fingerprint the member row holds.** There
//! is no certificate authority anywhere in a cluster, so the pinned fingerprint
//! is the whole of what says the machine that answered is the device this row is
//! about — see [`ThePinnedOne`], which is where a dial is refused rather than
//! after it. Nothing here trusts a host name or a chain: the name in a URL is
//! only where to knock, and the certificate that comes back is who answered.
//!
//! **Every address, in order, with a short patience apiece.** A member's row
//! holds every address that device advertised and the order it advertised them
//! in — the tailnet name and its addresses first, then the LAN — and a dial
//! works down that list until one answers. The address somebody typed at link
//! time is only the first one ever known: a laptop moves between the LAN and the
//! tailnet, and DHCP moves everybody. The patience is per address rather than
//! for the dial as a whole, which is what keeps a list whose first three
//! addresses are gone from costing the fourth anything — see [`REACHING`].
//!
//! **What a dial that got through writes is the member itself.** The moment it
//! answered, and the name, the OS and the addresses that machine now says it
//! has: this end cannot read another machine's hostname for itself, so what a
//! row holds is what the far end last said and a dial is when it says it again.
//! **And a dial that answered nowhere marks the member unreachable**, on that
//! first failure, leaving everything else about the row exactly as it stands —
//! the human settled that over a grace period, and a laptop with its lid shut is
//! a machine that is not there.
//!
//! **The identity endpoint is the dial everything else is proved by.** It asks
//! the caller for nothing, so it is the cheapest call a dial can be — and it is
//! also the one that refreshes a row, being where a device answers for itself.
//! Which is why it is worth making at all when this end already holds a row: the
//! answer is the row said again by the machine it is about.

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use verkstead_render::DeviceIdentity;
use verkstead_store::{Linking, Member};

use crate::device::{Device, fingerprint_of_der};
use crate::peer::{IDENTITY, Members, PEER_PORT};

/// How long one address has to answer the connection before the next one is
/// tried.
///
/// **Per address, which is the whole point of it.** A member's row is a list of
/// places that device has been, and the ones it is no longer at are the ones a
/// dial has to get past: a deadline over the dial as a whole would be a list
/// whose last address is never reached, and a device that moved is exactly the
/// device whose last address is the live one. So this is spent per attempt, and
/// a member with four addresses and one machine costs at most four of these
/// before it is reached.
///
/// Two seconds, because what is being waited out here is a connection — a `SYN`
/// to a machine that is not there any more, or a firewall dropping it — rather
/// than any work. Two seconds is longer than a handshake takes on any link two
/// devices could usefully be linked over, and short enough that a row of dead
/// addresses is a pause rather than a hang. The alternative is not a shorter
/// wait but the operating system's own, which is measured in minutes.
const REACHING: Duration = Duration::from_secs(2);

/// And how long the machine that answered has to finish answering.
///
/// Longer than [`REACHING`], and a different question: this end has a peer that
/// completed a handshake, so what is being waited on is the far end's own
/// reading of itself. Answering the identity endpoint runs `tailscale status` on
/// that machine — see [`crate::device::reading`], where it is given five seconds
/// of its own — so a deadline at [`REACHING`] would abandon a member that is
/// there and working.
///
/// Ten seconds, which is that reading and the request around it with room to
/// spare. It is spent at most once per dial, on the one address that answered.
const ANSWERING: Duration = Duration::from_secs(10);

/// The devices this one is linked to, as something to *call*: this device's
/// certificate to present, and the membership a dial's findings are written
/// into.
///
/// A handle rather than a function, because a dial is made of the two things
/// held here and neither of them is a caller's to pass in: what is presented is
/// this device's own identity, and where a row is refreshed or dimmed is the one
/// membership this Verkstead keeps.
#[derive(Debug, Clone)]
pub struct Peers {
    /// Whose certificate is presented — the outgoing one over a changeover,
    /// which is what every member holds.
    device: Device,

    /// And where what a dial found is written: the moment a member answered, or
    /// the mark that says it answered nowhere.
    members: Members,

    /// How long one address has to answer. [`REACHING`] on a running server.
    reaching: Duration,

    /// And how long the one that answered has to finish. [`ANSWERING`] on a
    /// running server.
    answering: Duration,
}

impl Peers {
    /// Dialling as this Verkstead does it: presenting `device`, recording into
    /// `members`.
    pub fn of(device: Device, members: Members) -> Peers {
        Peers {
            device,
            members,
            reaching: REACHING,
            answering: ANSWERING,
        }
    }

    /// The same, giving every address `patience` rather than the two deadlines a
    /// running server keeps.
    ///
    /// For a suite standing in front of a member whose addresses answer nothing:
    /// what is being asked there is that the patience is spent per address, and
    /// a test that waited out the real ones would be a test spending its time on
    /// the clock rather than on the question. One number for both, because the
    /// two are the same thing to a test — how long one address costs.
    pub fn waiting(self, patience: Duration) -> Peers {
        Peers {
            reaching: patience,
            answering: patience,
            ..self
        }
    }

    /// Read `member`'s identity off its peer listener, and write down what came
    /// of it.
    ///
    /// **Every address in the order the row holds them**, until one answers. A
    /// transport failure is this address being gone and nothing about the
    /// device, so the next one is tried; an address that answered *anything*
    /// ends the walk, because a second address of the same machine would answer
    /// the same way.
    ///
    /// Three things can come back. The member, which is recorded — the moment,
    /// and what that machine now says it is called, is running and can be
    /// reached on. A refusal, where the machine that answered is not this
    /// member: the row is left exactly as it stands, because nothing has been
    /// learned about the device it is about. Or nothing at all, which is the
    /// member marked unreachable.
    pub async fn identity(&self, member: &Member) -> Result<DeviceIdentity> {
        // Where the verifier writes the fingerprint it met when it was not the
        // one recorded, so that the two ways of reaching nobody can be told
        // apart down at the foot of this.
        let met = Arc::new(Mutex::new(None));

        let dialling = self.dialling(&member.fingerprint, &met)?;
        let mut nothing_at = Vec::new();

        for address in &member.addresses {
            let at = format!("https://{}{IDENTITY}", authority(address));

            match dialling.get(&at).send().await {
                Ok(answered) => return self.took(member, answered).await,

                Err(why) => {
                    tracing::debug!(
                        device = %member.device,
                        %address,
                        %why,
                        "a member did not answer at one of its addresses, so the next is tried",
                    );

                    nothing_at.push(address.as_str());
                }
            }
        }

        // A certificate met that was not the one recorded is a machine that
        // answered rather than a member that is not there — somebody else on
        // that address, or a device whose identity has been made again behind
        // this row's back. Neither is anything to dim a row over: the member is
        // where it was and this is a refusal, so the row is left alone and the
        // fingerprint that turned up is said.
        if let Some(met) = met.lock().expect("nothing panics holding this").take() {
            bail!(
                "device {} is recorded against {}, and the machine answering for it presented \
                 {met} instead",
                member.device,
                member.fingerprint,
            );
        }

        self.members.unreachable(&member.device).await?;

        // A member that advertised nothing has nowhere to be dialled at all,
        // which is the same finding reached by a shorter road: a device on
        // neither a tailnet nor a network answers with an empty list, and a row
        // holding one is a row nothing can get to.
        if nothing_at.is_empty() {
            bail!(
                "device {} advertised no address to dial it at",
                member.device
            );
        }

        tracing::info!(
            device = %member.device,
            addresses = %nothing_at.join(", "),
            "a member answered at none of its addresses, so it is drawn unreachable until \
             one of them does",
        );

        bail!(
            "device {} answered at none of the addresses it advertised ({})",
            member.device,
            nothing_at.join(", "),
        );
    }

    /// What a dial does with the answer it got: check that it is this member's,
    /// and write the member down as it now says it is.
    ///
    /// **The certificate was proved at the handshake** — see [`ThePinnedOne`],
    /// which refuses anything but the one this row holds — so what is left to
    /// check is the answer against it. A device names the fingerprint of the
    /// certificate it presented precisely so that a caller can hold the two up
    /// beside each other, and one that names another is not the device it says
    /// it is; the same goes for an answer under an id this row is not about.
    /// Neither is recorded, and neither dims the row: what answered is somebody
    /// else's business rather than a member that has gone.
    async fn took(&self, member: &Member, answered: reqwest::Response) -> Result<DeviceIdentity> {
        let status = answered.status();

        if !status.is_success() {
            bail!(
                "device {} answered {status} for its identity",
                member.device
            );
        }

        let identity: DeviceIdentity = answered
            .json()
            .await
            .with_context(|| format!("reading what device {} says it is", member.device))?;

        if identity.fingerprint != member.fingerprint {
            bail!(
                "device {} presented {} and named {} as its certificate, which is a device \
                 that is not the one it says it is",
                member.device,
                member.fingerprint,
                identity.fingerprint,
            );
        }

        if identity.device != member.device {
            bail!(
                "the device recorded as {} answers as {}, under the certificate recorded \
                 for the first of them",
                member.device,
                identity.device,
            );
        }

        self.members
            .refreshed(&Linking {
                device: member.device.clone(),
                name: identity.name.clone(),
                os: identity.os.clone(),
                addresses: identity.addresses.clone(),

                // The one recorded rather than the one answered, they having
                // just been checked against each other: what a row is keyed on
                // is a certificate this end has met, and the string in a payload
                // is the far end's word for it.
                fingerprint: member.fingerprint.clone(),
            })
            .await?;

        Ok(identity)
    }

    /// The client a dial is made with: this device's certificate to present, the
    /// far end's pinned to `expecting`, and a deadline apiece on reaching a
    /// machine and on being answered by one.
    ///
    /// Built per dial rather than held, because the pinning is per member: what
    /// makes this configuration worth anything is that it will complete a
    /// handshake with one certificate and no other, and a client shared between
    /// two members could only ever be pinned to one of them.
    fn dialling(
        &self,
        expecting: &str,
        met: &Arc<Mutex<Option<String>>>,
    ) -> Result<reqwest::Client> {
        let pem = self.device.certificate().as_bytes();

        let key = PrivateKeyDer::from_pem_slice(pem)
            .context("reading this device's private key to dial a peer with")?;
        let certificate = CertificateDer::from_pem_slice(pem)
            .context("reading this device's certificate to present to a peer")?;

        // `ring`, named rather than taken from rustls' own default, for the
        // reason the listener names it — see [`super::presenting`].
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let algorithms = provider.signature_verification_algorithms;

        let dialling = ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .context("settling the protocol versions a peer is dialled over")?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(ThePinnedOne {
                expecting: expecting.to_owned(),
                met: Arc::clone(met),
                algorithms,
            }))
            .with_client_auth_cert(vec![certificate], key)
            .context("presenting this device's certificate to a peer")?;

        reqwest::Client::builder()
            .use_preconfigured_tls(dialling)
            // The connection and the answer are two deadlines because they are
            // two different waits — see [`REACHING`] and [`ANSWERING`]. The
            // first is what a dead address costs, and it is spent once per
            // address; the second is what the machine that answered is given,
            // and it is spent once.
            .connect_timeout(self.reaching)
            .timeout(self.answering)
            .build()
            .context("building the client a peer is dialled with")
    }
}

/// Where a bare address is dialled: the peer port, unless the address says
/// otherwise.
///
/// A device advertises addresses rather than sockets — what it says of itself is
/// where it is, and the port it answers on is [`PEER_PORT`] on every
/// installation that has not been told another. So a bare address, of either
/// family, is dialled there.
///
/// **And an address that carries a port is dialled at it**, because the port is
/// something a host may be told: `peer_listen` is what a NixOS host says the
/// listener is on, so a device that is not on the default one is reachable only
/// by an address that says which — and an address with a port on it is also the
/// shape of the one somebody types when they link two machines.
fn authority(address: &str) -> String {
    // A bare IP, which is most of what a device advertises. The brackets are
    // what makes a v6 address a URL authority rather than an address with a
    // port already in it.
    if let Ok(address) = address.parse::<IpAddr>() {
        return match address {
            IpAddr::V4(address) => format!("{address}:{PEER_PORT}"),
            IpAddr::V6(address) => format!("[{address}]:{PEER_PORT}"),
        };
    }

    // An address with a port on it already, of either family: `1.2.3.4:8423`,
    // or a bracketed v6 one.
    if address.parse::<SocketAddr>().is_ok() {
        return address.to_owned();
    }

    // And a name with a port on it, which is neither of the above and is what
    // somebody types. A name with a colon and no number after it is a name
    // rather than a port, and goes to the peer port with the rest.
    if address
        .rsplit_once(':')
        .is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok())
    {
        return address.to_owned();
    }

    format!("{address}:{PEER_PORT}")
}

/// The far end's half of *a member is a fingerprint*: the certificate recorded
/// for this member is accepted, and every other certificate is refused.
///
/// **This is where a dial is proved rather than after it.** There is no
/// authority anywhere in a cluster to check a chain against, so the pinned
/// fingerprint is the whole of what says the machine that answered is the device
/// the row is about — and a check made after the handshake would be a request
/// this end had already sent to somebody it had not identified.
///
/// Nothing is asked about a host name, and nothing about a chain. The name a
/// dial goes out under is where to knock: an address out of a member's row, or a
/// tailnet name, neither of which is in anybody's certificate.
#[derive(Debug)]
struct ThePinnedOne {
    /// The fingerprint recorded for this member, which is the one certificate
    /// that gets through.
    expecting: String,

    /// And where the one that turned up is left when it was not that: what tells
    /// *somebody else answered* from *nobody answered*, which are two different
    /// things to say about a member and only one of them dims its row.
    met: Arc<Mutex<Option<String>>>,

    /// The signature algorithms the provider supports, which is what the two
    /// signature checks below are run against.
    algorithms: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for ThePinnedOne {
    /// The certificate itself, compared as a fingerprint — which is the spelling
    /// a member row keeps one in, so that the comparison is between two strings
    /// about one certificate rather than between two spellings of it.
    ///
    /// The validity dates are not asked about here, and the reason is the one
    /// [`super::WhateverArrives`] gives for asking them: an expired certificate
    /// is refused at the *listener*, which is where a certificate presented to
    /// somebody is judged. What this end is doing is recognising a certificate
    /// it has already met and written down, and a member whose certificate has
    /// run out is a member whose renewal is the thing to announce — see
    /// [`crate::device::Changeover`] — rather than a dial to refuse locally.
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let met = fingerprint_of_der(end_entity);

        if met != self.expecting {
            *self.met.lock().expect("nothing panics holding this") = Some(met);

            return Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ));
        }

        Ok(ServerCertVerified::assertion())
    }

    /// And that the far end holds the key that signed what it presented, on a
    /// TLS 1.2 handshake: without this a certificate would be a public file
    /// anybody who had read one could replay, and the fingerprint above would
    /// admit whoever had.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    /// And the same on a TLS 1.3 one, which is what two devices of this build
    /// negotiate.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A bare address of either family is dialled on the peer port, and the v6
    /// one is bracketed — an address with colons in it and a port after it is
    /// not a URL authority anybody could read.
    #[test]
    fn a_bare_address_is_dialled_on_the_peer_port() {
        assert_eq!(
            authority("192.168.1.24"),
            format!("192.168.1.24:{PEER_PORT}")
        );
        assert_eq!(authority("100.64.0.1"), format!("100.64.0.1:{PEER_PORT}"));
        assert_eq!(
            authority("fd7a:115c:a1e0::1"),
            format!("[fd7a:115c:a1e0::1]:{PEER_PORT}"),
        );
    }

    /// And so is a tailnet name, which is the first thing on every list a
    /// machine on one advertises.
    #[test]
    fn a_name_is_dialled_on_the_peer_port_too() {
        assert_eq!(
            authority("workbench.tailnet-name.ts.net"),
            format!("workbench.tailnet-name.ts.net:{PEER_PORT}"),
        );
    }

    /// And an address that already says which port is dialled there: the port a
    /// device answers on is something a host may be told.
    #[test]
    fn an_address_that_names_a_port_is_dialled_at_it() {
        assert_eq!(authority("192.168.1.24:9000"), "192.168.1.24:9000");
        assert_eq!(
            authority("[fd7a:115c:a1e0::1]:9000"),
            "[fd7a:115c:a1e0::1]:9000"
        );
        assert_eq!(
            authority("workbench.tailnet-name.ts.net:9000"),
            "workbench.tailnet-name.ts.net:9000",
        );
    }
}
