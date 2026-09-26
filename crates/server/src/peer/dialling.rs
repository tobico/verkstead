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
//! **With two exceptions, and both of them are dials to a stranger.** A device
//! pressing Add has never met the machine it is asking, so there is no
//! fingerprint to pin it on: that dial takes whatever certificate turns up, for
//! the one call, and hands back what it turned out to be — which is the string
//! the two humans then compare by eye and the string every dial after it is
//! pinned on. The tailnet half of a discovery is the other, and for the same
//! reason: it asks a node nothing has heard of what it is, and a probe that
//! insisted on a fingerprint could only ever find devices already linked. See
//! [`Peers::join`], [`Peers::stranger`] and [`WhateverIsThere`].
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

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{WebPkiSupportedAlgorithms, verify_tls12_signature, verify_tls13_signature};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use verkstead_render::{DeviceIdentity, DiscoveredDevice, JoinHeld, JoinSettled};
use verkstead_store::{HeldJoin, Linking, Member};

use crate::device::{Device, fingerprint_of_der};
use crate::peer::exchange::settling;
use crate::peer::joining::{JOIN, cancelling};
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

/// The most of an answer this device will read off a **stranger**: **64 KiB**.
///
/// **Because the tailnet half of a discovery dials machines nobody typed an
/// address for** — see [`Peers::stranger`]. Every other bound on a probe is on
/// how many nodes are asked and how long one has to answer; without this one, a
/// node that answers slowly and endlessly is a read with no end but a deadline,
/// and there are [`crate::discovery::AT_ONCE`] of them in flight at a time. It is
/// the outbound half of the bound a join already has coming the other way, where
/// what a stranger may post into this machine is capped by the listener itself.
///
/// Sixty-four kibibytes because an identity is nowhere near it: the longest one a
/// device may say — see [`crate::peer::joining::too_much`] — is a name, an OS
/// word, an id, a fingerprint and sixteen addresses, which is some four thousand
/// characters. So this is roomy enough that no Verkstead ever meets it, and small
/// enough that sixteen at once is nothing. Anything over it is a node that is not
/// a Verkstead, which is what most of a tailnet is.
pub const MOST_SAID: usize = 64 * 1024;

/// What a press on a **Discovered** row came to — see [`Peers::join_found`],
/// whose whole answer this is.
///
/// **The two are worth telling apart at the end of the walk as well as at each
/// address of it**, which is why this stands beside [`Knocked`] rather than
/// being that enum said twice: a device that answered *anything* is a device
/// that is where the row said it was, and a row nothing answered at is a row
/// that was wrong. The caller does different things with the two — see
/// [`crate::device::Devices::add_found`], which stops drawing the row for the
/// second and leaves it alone for the first.
pub enum Reached {
    /// A device answered at one of the row's addresses, and this is what came of
    /// it: the address that answered with the question now held over there, or
    /// this device's account of why it is not held. Either way that device is
    /// there.
    Answered(Result<(String, JoinHeld)>),

    /// And nothing answered at any of them, which is a row that went stale
    /// between being drawn and being pressed.
    Nobody(anyhow::Error),
}

/// What one knock at one address came to — see [`Peers::knocked`], and
/// [`Peers::join_found`], which is the one caller that cares which of the two it
/// is.
enum Knocked {
    /// The address answered, and this is what came of the answer: the question
    /// held over there, or this device's account of why it was not. Either way
    /// the device has spoken, and a walk down its other addresses is over.
    Answered(Result<JoinHeld>),

    /// And nothing at that address at all, which is the one finding worth trying
    /// another address for: a machine that has moved, or a row drawn of a device
    /// that has since gone.
    Nobody(anyhow::Error),
}

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

    /// The client each member's **relayed** traffic goes over, held so that a
    /// second call reuses the connection the first one opened — see
    /// [`Held`] and [`Peers::held_for`].
    ///
    /// Shared across every clone of this handle, because the point of it is
    /// that there is one pool per member rather than one per caller: the hop
    /// dials from whichever request task the browser landed in, and a cache
    /// that cloned would be no cache at all.
    ///
    /// **Relayed traffic alone.** Every other dial here happens now and then —
    /// an identity read, a telling, a join — and builds its client as it always
    /// did: a client apiece costs nothing at that rate, and each of those walks
    /// wants its own reading of what it met.
    holding: Arc<Mutex<HashMap<String, Held>>>,
}

/// One member's dialling client, and what it was built against.
///
/// **Rebuilt rather than refreshed when either end's certificate moves.** A
/// client presents this device's own certificate and pins the member's, and
/// both are re-issued from time to time — a renewal here, a changeover there —
/// so what is kept beside the client is the fingerprints it was made for. One
/// that no longer matches is a client that would present something the far end
/// has never acknowledged, or pin something that member no longer holds.
///
/// There is an entry per member ever dialled and nothing prunes them: a cluster
/// is a handful of devices, an unlinked one leaves a client whose idle
/// connections reqwest reaps on its own, and a device linked again lands on its
/// own key.
#[derive(Debug)]
struct Held {
    /// The fingerprints this client accepts from the far end — the member's,
    /// and the one it is changing over from where it is.
    expecting: Vec<String>,

    /// And the fingerprint of this device's own certificate it presents.
    presenting: String,

    /// Where a certificate met that was not one of `expecting` is written,
    /// shared with the verifier inside the client for as long as it lives.
    ///
    /// Which is why [`Peers::held_for`] empties it before handing it back: it
    /// holds whatever the *last* walk met, and a walk that reaches nobody says
    /// whatever is in it — so one carried over would have this device reporting
    /// a mismatch from ten minutes ago as the reason a member is not there.
    met: Arc<Mutex<Option<String>>>,

    client: reqwest::Client,
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
            holding: Arc::new(Mutex::new(HashMap::new())),
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

        let dialling = self.dialling(accepted(member), &met)?;
        let mut nothing_at = Vec::new();

        for address in &member.addresses {
            let at = reaching(address, IDENTITY);

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

        Err(self.nowhere(member, &met, &nothing_at).await)
    }

    /// What a walk that reached nobody came to, in the words the caller hands
    /// back — and the row dimmed where that is what it means.
    ///
    /// **One reading rather than one per thing a dial is for.** The three walks
    /// here — the identity read, the tellings, and a relayed call — end the
    /// same way whatever they were carrying, because what they ran out of is
    /// the same thing: a member's addresses.
    ///
    /// A certificate met that was not the one recorded is a machine that
    /// answered rather than a member that is not there — somebody else on that
    /// address, or a device whose identity has been made again behind this
    /// row's back. Neither is anything to dim a row over: the member is where
    /// it was and this is a refusal, so the row is left alone and the
    /// fingerprint that turned up is said.
    ///
    /// And a member that advertised nothing has nowhere to be dialled at all,
    /// which is the same finding reached by a shorter road: a device on neither
    /// a tailnet nor a network answers with an empty list, and a row holding
    /// one is a row nothing can get to.
    async fn nowhere(
        &self,
        member: &Member,
        met: &Arc<Mutex<Option<String>>>,
        nothing_at: &[&str],
    ) -> anyhow::Error {
        if let Some(met) = met.lock().expect("nothing panics holding this").take() {
            return anyhow!(
                "device {} is recorded against {}, and the machine answering for it presented \
                 {met} instead",
                member.device,
                member.fingerprint,
            );
        }

        if let Err(why) = self.members.unreachable(&member.device).await {
            return why;
        }

        if nothing_at.is_empty() {
            return anyhow!(
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

        anyhow!(
            "device {} answered at none of the addresses it advertised ({})",
            member.device,
            nothing_at.join(", "),
        )
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

        // Either of the two a member in the middle of a changeover has, which is
        // the same pair its handshake was just accepted against — see
        // [`accepted`]. What is being asked is that the machine naming itself is
        // the machine that presented, and both of those certificates are that
        // machine's.
        if !accepted(member)
            .iter()
            .any(|accepted| accepted == &identity.fingerprint)
        {
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

        // And where this member was in the middle of a changeover and has
        // answered under the certificate it was changing *to*, that changeover is
        // over as far as this device is concerned: the far end has stopped
        // presenting the other one, which is the only thing that was ever going
        // to say so. Written here because this is the one call that reads what a
        // member says it is — see [`verkstead_store::Member::renewing_from`].
        if member.renewing_from.is_some() && identity.fingerprint == member.fingerprint {
            self.members.changed_over(&member.device).await?;
        }

        Ok(identity)
    }

    /// Tell `member` about `newcomer`, which is what an introducer does the
    /// moment its human has pressed Allow.
    ///
    /// **An ordinary member's call, pinned like every other.** The far end is
    /// a device this one has confirmed and holds a certificate for, so the
    /// route answering this stands behind that end's own member gate — see
    /// [`crate::peer::announcing`]. What makes the claim worth recording over
    /// there is precisely that it came down a link the receiver has verified:
    /// the newcomer itself could say the same words and be refused for a
    /// stranger, which is what the announcement being the introducer's is for.
    ///
    /// **Every address in the order the row holds them**, until one answers, as
    /// [`Peers::identity`] works down the same list — and the member that
    /// answered nowhere is marked unreachable by the same judgement: a dial that
    /// reached none of a device's addresses is a machine that is not there.
    ///
    /// What is *not* here is a retry. The failure is handed back to the caller,
    /// which writes the telling down as owed and carries on: a human pressed
    /// Allow, and a laptop with its lid shut is that machine's problem rather
    /// than the press's.
    pub async fn announce(&self, member: &Member, newcomer: &DeviceIdentity) -> Result<()> {
        self.telling(
            member,
            &format!("being told about device {}", newcomer.device),
            |dialling, at| {
                dialling
                    .post(reaching(at, crate::peer::announcing::MEMBERS))
                    .json(newcomer)
            },
        )
        .await
    }

    /// And tell `member` that `leaving` is not one of this cluster's any more,
    /// which is what an Unlink broadcasts.
    ///
    /// **The announcement above, the other way round**, and a member's call in
    /// exactly the same sense: it goes down a link the far end has verified, and
    /// nobody over there is asked to confirm it — a cluster is a membership, so
    /// the press that dropped the device dropped it for everybody.
    ///
    /// **And the leaver is told over this same call**, with its own id as
    /// `leaving`: what a device does when the id named is its own is let go of
    /// every member it holds. See [`crate::peer::unlinking`], which is what
    /// answers.
    ///
    /// The same walk down the addresses and the same reading of a member that
    /// answers nowhere. What is not here is a retry: the failure is handed back
    /// to the caller, which writes the removal down as owed — a member that was
    /// off when the human pressed Unlink is told when it next answers.
    pub async fn unlink(&self, member: &Member, leaving: &str) -> Result<()> {
        self.telling(
            member,
            &format!("being told to drop device {leaving}"),
            |dialling, at| {
                dialling.delete(reaching(
                    at,
                    &crate::peer::unlinking::MEMBER.replace("{device}", leaving),
                ))
            },
        )
        .await
    }

    /// And tell `member` that this device has made its certificate again, which
    /// is what a start that re-issued one owes every one of them.
    ///
    /// **Made presenting the outgoing certificate, which is not a choice.** The
    /// certificate this device presents over a changeover *is* the outgoing one —
    /// see [`Device::certificate`] — and it has to be: the new one is a
    /// certificate no member holds, so a call made under it would be refused at
    /// every gate in the cluster, including by the very device being told about
    /// it. Which is the whole reason the changeover has two certificates and not
    /// one.
    ///
    /// **And a member's own call like the two above it.** It goes down a link the
    /// far end has verified and nobody over there presses anything: the id it
    /// arrives under is one that end's human allowed once already, and what is
    /// changing is which certificate stands against it — see
    /// [`crate::peer::renewing`], which is what answers.
    ///
    /// The same walk down the addresses and the same reading of a member that
    /// answers nowhere. What is not here is a retry: the failure is handed back to
    /// the caller, which writes the telling down as owed — a member that was off
    /// when the certificate was made again is told when it next answers, and the
    /// changeover waits for it rather than costing a call in the meantime.
    ///
    /// **The answer is the acknowledgement**, so `Ok(())` is what takes this
    /// member off the list the changeover is waiting on. Nothing is read out of
    /// the body: what this end needed to know is that the call landed.
    pub async fn announce_renewal(
        &self,
        member: &Member,
        renewed: &verkstead_render::RenewedCertificate,
    ) -> Result<()> {
        self.telling(
            member,
            &format!(
                "being told this device is changing over to certificate {}",
                renewed.incoming,
            ),
            |dialling, at| {
                dialling
                    .post(reaching(at, crate::peer::renewing::CERTIFICATE))
                    .json(renewed)
            },
        )
        .await
    }

    /// What all three of those are: one call made to a member, at every address
    /// the row holds and in the order it holds them, until one answers.
    ///
    /// **One walk rather than one per thing said.** What differs between an
    /// announcement and an unlink is the request built and the words a refusal
    /// is reported in; everything else — the pinned certificate, the addresses
    /// worked down, the machine that answered with somebody else's certificate,
    /// the member that answered nowhere being dimmed — is what *dialling a
    /// member* means, and it is one behaviour however many things there are to
    /// say over it.
    ///
    /// `doing` is what the far end did not do, in the words a log line reads
    /// it in: *being told about device …*, *being told to drop device …*.
    ///
    /// A transport failure is that address being gone and nothing about the
    /// device, so the next one is tried; an address that answered *anything*
    /// ends the walk, because a second address of the same machine would answer
    /// the same way.
    async fn telling(
        &self,
        member: &Member,
        doing: &str,
        build: impl Fn(&reqwest::Client, &str) -> reqwest::RequestBuilder,
    ) -> Result<()> {
        let met = Arc::new(Mutex::new(None));
        let dialling = self.dialling(accepted(member), &met)?;
        let mut nothing_at = Vec::new();

        for address in &member.addresses {
            match build(&dialling, address).send().await {
                Ok(answered) => {
                    let status = answered.status();

                    if !status.is_success() {
                        let said = answered.text().await.unwrap_or_default();

                        bail!(
                            "device {} answered {status} to {doing}: {}",
                            member.device,
                            said.trim(),
                        );
                    }

                    return Ok(());
                }

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

        Err(self.nowhere(member, &met, &nothing_at).await)
    }

    /// Put `call` to `member` and hand back what it answered, which is the hop
    /// a relayed call is made over (ADR-0020, *The opened device relays*) — see
    /// [`crate::relaying`], which is the end of it the browser reaches.
    ///
    /// **An ordinary member's call, pinned like every other**, and the same
    /// walk down the same addresses. What is different is that it carries
    /// somebody else's request rather than something this device had to say,
    /// so nothing is read out of the answer here: the status, the headers and
    /// the body are the browser's and are handed straight back.
    ///
    /// **And a body cannot be sent twice**, which is what shortens the walk.
    /// The other dials here try the next address on any failure; this one tries
    /// it only where the attempt failed to *connect*, that being the one
    /// failure that happens before a single byte of the body has been asked for
    /// — see [`crate::relaying::Streamed`]. A machine that answered the
    /// handshake and then stopped is a machine that has had part of the
    /// request, and there is nothing left to send it to the next address.
    ///
    /// **No deadline on the answer**, unlike every other dial here — see
    /// [`Peers::relaying`]. The address still has [`REACHING`] to answer the
    /// connection, so a member that has moved costs what it always did.
    ///
    /// **And a socket is one of the answers.** Three of the endpoints in that
    /// namespace answer `101` rather than a body, and this is the dial that asks
    /// them: what comes back is handed to [`crate::relaying::bridging`], which
    /// takes the connection out of it and joins it to the browser's. Nothing here
    /// is different for one — it is the same request under the same certificate,
    /// and the deadline this dial does without is what lets the connection live
    /// as long as the pane does.
    pub(crate) async fn relay(
        &self,
        member: &Member,
        call: &crate::relaying::Call,
    ) -> Result<reqwest::Response> {
        let (dialling, met) = self.held_for(member)?;
        let mut nothing_at = Vec::new();

        for address in &member.addresses {
            let mut attempt = dialling
                .request(call.method.clone(), reaching(address, &call.onwards))
                .headers(call.headers.clone());

            if let Some(body) = call.body.for_one_attempt() {
                attempt = attempt.body(body);
            }

            match attempt.send().await {
                Ok(answered) => return Ok(answered),

                Err(why) if why.is_connect() => {
                    tracing::debug!(
                        device = %member.device,
                        %address,
                        %why,
                        "a member did not answer at one of its addresses, so the next is tried",
                    );

                    nothing_at.push(address.as_str());
                }

                Err(why) => {
                    bail!(
                        "device {} stopped answering at {address} partway through a call this \
                         device was relaying to it: {why}",
                        member.device,
                    );
                }
            }
        }

        Err(self.nowhere(member, &met, &nothing_at).await)
    }

    /// Ask the device at `address` to let this one into its cluster, saying
    /// `saying` of itself — which is the press on **Add**.
    ///
    /// **Whatever certificate that address presents is taken, for this one
    /// call.** There is nothing yet by which to know what the far end's
    /// certificate ought to be: that is exactly what the two humans are about to
    /// confirm by eye, and a dial that insisted on a fingerprint first would be
    /// a link that could only be made between devices already linked. So this is
    /// the one call in this module that is not pinned — see [`WhateverIsThere`],
    /// which takes what turns up and says what it was.
    ///
    /// **And what turned up is checked against what the far end says it is.** A
    /// device names the fingerprint of the certificate it presented so that a
    /// caller can hold the two up beside each other, and one that names another
    /// is not the device it says it is — the same judgement [`Peers::took`]
    /// makes of an identity answer, which is the whole of what can be asked of a
    /// stranger.
    ///
    /// The one address rather than a list, because there is no list: what the
    /// human typed is the only place this device has been told to look, and
    /// every address the far end has is in the answer — which is what a link
    /// carries from here on. A device a discovery *found* holds a list of them,
    /// and that is [`Peers::join_found`].
    pub async fn join(&self, address: &str, saying: &DeviceIdentity) -> Result<JoinHeld> {
        match self.knocked(address, saying).await {
            Knocked::Answered(held) => held,
            Knocked::Nobody(why) => Err(why),
        }
    }

    /// The same question put to a device a discovery found, which is a **list**
    /// of addresses rather than one (ADR-0020, *Discovery*) — and the address it
    /// turned out to answer at, which is what the pending row is left naming.
    ///
    /// **Every address in the order the row holds them, until one answers**,
    /// exactly as a dial to a member works down that member's — see
    /// [`Peers::identity`], which is this walk made against a device this end
    /// already knows. The row holds the LAN's addresses before the tailnet's, so
    /// the shorter road is the one tried first.
    ///
    /// **An address that answered ends the walk, whatever it answered.** A second
    /// address of the same machine would answer the same way, and a refusal asked
    /// twice would be two questions held over there for one press — which is the
    /// one thing this walk must not leave behind.
    ///
    /// **And a row can be stale by the time it is pressed.** A device may have
    /// gone off the LAN or left the tailnet between the browse finding it and
    /// somebody pressing Add, so a walk that reached nobody is refused in the
    /// words a dial that reached nobody uses, naming the device the row drew.
    ///
    /// **Which is why the answer is a [`Reached`] rather than a bare result.** A
    /// walk that reached nobody and a device that answered and said no are two
    /// findings rather than one failure — the first is about the row and the
    /// second is about the far end — and only the first is a reason to stop
    /// drawing the row.
    pub async fn join_found(&self, found: &DiscoveredDevice, saying: &DeviceIdentity) -> Reached {
        let mut nothing_at = Vec::new();

        for address in &found.addresses {
            match self.knocked(address, saying).await {
                Knocked::Answered(held) => {
                    return Reached::Answered(held.map(|held| (address.clone(), held)));
                }

                Knocked::Nobody(why) => {
                    tracing::debug!(
                        device = %found.device,
                        %address,
                        why = format!("{why:#}"),
                        "a device this one found did not answer at one of the addresses it was \
                         found at, so the next is tried",
                    );

                    nothing_at.push(address.as_str());
                }
            }
        }

        // A row with no address at all is the same finding by a shorter road, and
        // it is the one [`Peers::identity`] makes of a member that advertised
        // nothing: there is nowhere to knock.
        if nothing_at.is_empty() {
            return Reached::Nobody(anyhow!(
                "{} was found at no address to dial it at",
                found.name,
            ));
        }

        Reached::Nobody(anyhow!(
            "{} answered at none of the addresses it was found at ({})",
            found.name,
            nothing_at.join(", "),
        ))
    }

    /// One knock at one address: what came of it, or nobody there at all.
    ///
    /// The two are worth telling apart, which is the whole reason this is not
    /// simply [`Peers::join`]'s body: a transport failure is that address being
    /// gone and says nothing about the device, where anything the far end *said*
    /// is about the device and ends a walk down its addresses.
    async fn knocked(&self, address: &str, saying: &DeviceIdentity) -> Knocked {
        let met = Arc::new(Mutex::new(None));

        let asking = match self.asking(&met) {
            Ok(asking) => asking,

            // Not an address that answered nothing: a client this device cannot
            // build is this device's own failure, and trying the next address
            // would fail it again.
            Err(why) => return Knocked::Answered(Err(why)),
        };

        let answered = match asking
            .post(reaching(address, JOIN))
            .json(saying)
            .send()
            .await
        {
            Ok(answered) => answered,

            Err(why) => {
                return Knocked::Nobody(anyhow::Error::new(why).context(format!(
                    "asking the device at {address} to link with this one"
                )));
            }
        };

        Knocked::Answered(self.taken(address, answered, &met).await)
    }

    /// And what an answer to a join post was worth: the question held over there,
    /// or this device's account of why it was not.
    async fn taken(
        &self,
        address: &str,
        answered: reqwest::Response,
        met: &Arc<Mutex<Option<String>>>,
    ) -> Result<JoinHeld> {
        let status = answered.status();

        if !status.is_success() {
            let said = answered.text().await.unwrap_or_default();

            bail!(
                "the device at {address} answered {status} to a request to link: {}",
                said.trim(),
            );
        }

        let held: JoinHeld = answered
            .json()
            .await
            .with_context(|| format!("reading what the device at {address} said to the request"))?;

        let met = met
            .lock()
            .expect("nothing panics holding this")
            .take()
            .with_context(|| format!("the device at {address} presented no certificate"))?;

        if held.identity.fingerprint != met {
            bail!(
                "the device at {address} presented {met} and named {} as its certificate, \
                 which is a device that is not the one it says it is",
                held.identity.fingerprint,
            );
        }

        Ok(held)
    }

    /// What the device at `address` says it is, when this one has never heard of
    /// it — which is the tailnet half of a discovery asking a peer what it is
    /// (ADR-0020, *Discovery*).
    ///
    /// **Whatever certificate that address presents is taken, for this one
    /// call**, exactly as [`Peers::join`] takes one and for the same reason:
    /// there is nothing yet by which to know what the far end's certificate
    /// ought to be, and a probe that insisted on a fingerprint could only ever
    /// find devices already linked. What is on the row this fills is a name, a
    /// mark and an address — nothing anybody is asked to trust — and the
    /// fingerprint two people compare by eye arrives on the pending row that the
    /// press on **Add** leaves.
    ///
    /// **And what turned up is checked against what the far end says it is**, the
    /// one judgement that can be made of a stranger: a device names the
    /// fingerprint of the certificate it presented so that a caller can hold the
    /// two up beside each other, and one that names another is not the device it
    /// says it is. So is one that answers something which is not an identity at
    /// all, which is most of what is on a tailnet — a phone, a server, anything
    /// with something else on that port.
    ///
    /// **Nothing is recorded and nothing is dimmed.** The far end is not a
    /// member: there is no row to write a finding on to, and a device that
    /// answered nothing is a row that is simply not drawn.
    ///
    /// **And what it will read of the answer is bounded** — see [`MOST_SAID`].
    /// Every other bound on a probe is on how many of these are made and how long
    /// one may take; this is the bound on what one of them may *cost*, and it is
    /// the half that has to be here rather than at the caller, a body being read
    /// where it is read.
    ///
    /// The one address rather than a list, because the caller is working down a
    /// list of its own — see [`crate::discovery::Probe`], which spends a deadline
    /// per peer rather than per address.
    pub async fn stranger(&self, address: &str) -> Result<DeviceIdentity> {
        let met = Arc::new(Mutex::new(None));
        let asking = self.asking(&met)?;
        let at = reaching(address, IDENTITY);

        let mut answered = asking
            .get(&at)
            .send()
            .await
            .with_context(|| format!("asking the device at {address} what it is"))?;

        let status = answered.status();

        if !status.is_success() {
            bail!("the device at {address} answered {status} for its identity");
        }

        // Read in the chunks it arrives in rather than whole, which is the only
        // way the bound is one on what is held: a `Content-Length` is the far
        // end's own word for how much it is about to send.
        let mut said = Vec::new();

        while let Some(chunk) = answered
            .chunk()
            .await
            .with_context(|| format!("reading what the device at {address} says it is"))?
        {
            if said.len() + chunk.len() > MOST_SAID {
                bail!(
                    "the device at {address} said more about itself than an identity is, so it \
                     is not one",
                );
            }

            said.extend_from_slice(&chunk);
        }

        let identity: DeviceIdentity = serde_json::from_slice(&said)
            .with_context(|| format!("reading what the device at {address} says it is"))?;

        let met = met
            .lock()
            .expect("nothing panics holding this")
            .take()
            .with_context(|| format!("the device at {address} presented no certificate"))?;

        if identity.fingerprint != met {
            bail!(
                "the device at {address} presented {met} and named {} as its certificate, \
                 which is a device that is not the one it says it is",
                identity.fingerprint,
            );
        }

        Ok(identity)
    }

    /// And take that question back, which is Cancel on the pending row.
    ///
    /// **Pinned, where the join was not.** By the time there is anything to
    /// cancel this device has met the far end's certificate and written it down,
    /// so a cancel is an ordinary dial against a fingerprint it holds — and it
    /// has to be, or a machine that had taken that address since would be told
    /// which link this device was in the middle of making.
    pub async fn cancel(&self, address: &str, request: &str, expecting: &str) -> Result<()> {
        let met = Arc::new(Mutex::new(None));
        let dialling = self.dialling(vec![expecting.to_owned()], &met)?;
        let at = reaching(address, &cancelling(request));

        let answered = dialling
            .post(&at)
            .send()
            .await
            .with_context(|| format!("taking back the request to link with {address}"))?;

        let status = answered.status();

        if !status.is_success() {
            let said = answered.text().await.unwrap_or_default();

            bail!(
                "the device at {address} answered {status} to the request being taken back: {}",
                said.trim(),
            );
        }

        Ok(())
    }

    /// And the dial back: tell the device that asked what came of its request,
    /// handing over the roster where it was let in.
    ///
    /// **Pinned on the certificate the request is holding**, which is what
    /// makes this safe to make to a device that is not a member yet. The join
    /// post arrived over a handshake and the certificate it presented was
    /// written into the request; whatever is at those addresses now has to turn
    /// out to be presenting the same one, or the handshake does not complete
    /// and nothing of this cluster is said to it. A device that answers on the
    /// asker's address with some other certificate is not the device that
    /// asked, and the exchange stops here rather than at the far end.
    ///
    /// **Every address the request holds, in the order they were advertised**,
    /// exactly as a dial to a member works down that member's — see
    /// [`Peers::identity`]. A laptop that moved between posting the join and
    /// its human's opposite number pressing Allow is reached at the next
    /// address on the list, and the ten minutes are long enough for that to be
    /// an ordinary thing to happen.
    ///
    /// Nothing is recorded here and nothing is dimmed. The device at the far
    /// end is not a member — it is the thing this call is about to make one, or
    /// about to tell there will be none — so there is no row to write a finding
    /// on to.
    pub async fn settle(&self, held: &HeldJoin, settled: &JoinSettled) -> Result<()> {
        let met = Arc::new(Mutex::new(None));
        let dialling = self.dialling(vec![held.fingerprint.clone()], &met)?;
        let path = settling(&held.request);
        let mut nothing_at = Vec::new();

        for address in &held.addresses {
            let at = reaching(address, &path);

            match dialling.post(&at).json(settled).send().await {
                Ok(answered) => {
                    let status = answered.status();

                    if !status.is_success() {
                        let said = answered.text().await.unwrap_or_default();

                        bail!(
                            "device {} answered {status} to being told what came of its \
                             request to link: {}",
                            held.device,
                            said.trim(),
                        );
                    }

                    return Ok(());
                }

                Err(why) => {
                    tracing::debug!(
                        device = %held.device,
                        %address,
                        %why,
                        "a device that asked to link did not answer at one of the addresses \
                         it advertised, so the next is tried",
                    );

                    nothing_at.push(address.as_str());
                }
            }
        }

        // A certificate met that was not the one the request pinned is a
        // machine answering at that address rather than the device that asked,
        // and it is worth saying apart from silence: one is somebody else on an
        // address a laptop has left, and the other is a laptop that is off.
        if let Some(met) = met.lock().expect("nothing panics holding this").take() {
            bail!(
                "device {} asked under {}, and the machine answering at its addresses \
                 presented {met} instead",
                held.device,
                held.fingerprint,
            );
        }

        if nothing_at.is_empty() {
            bail!(
                "device {} advertised no address to answer it at",
                held.device
            );
        }

        bail!(
            "device {} answered at none of the addresses it advertised ({})",
            held.device,
            nothing_at.join(", "),
        );
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
        expecting: Vec<String>,
        met: &Arc<Mutex<Option<String>>>,
    ) -> Result<reqwest::Client> {
        self.client(Some(self.answering), |algorithms| {
            Arc::new(ThePinnedOne {
                expecting,
                met: Arc::clone(met),
                algorithms,
            })
        })
    }

    /// The client `member`'s relayed traffic goes over, and where a mismatch
    /// met on the way is written — built the first time and held after it, so
    /// that a second call reuses the connection the first one opened.
    ///
    /// **Because a relay is traffic rather than an errand.** Every other dial
    /// here happens now and then; this one is the browser, and a remote
    /// Conversation is a page load's worth of calls with a Code pane putting
    /// one more on it per folder opened and per file read. A client apiece
    /// meant this device's certificate re-parsed, a rustls config built and a
    /// whole TLS handshake for each of them — over a tailnet, the handshake
    /// *is* the page load.
    ///
    /// **Rebuilt where either certificate has moved**, which is the whole of
    /// what makes holding one safe — see [`Held`]. And what it holds of the
    /// last walk is emptied on the way out, for the reason [`Held::met`] gives.
    fn held_for(&self, member: &Member) -> Result<(reqwest::Client, Arc<Mutex<Option<String>>>)> {
        let expecting = accepted(member);
        let presenting = self.device.fingerprint().to_owned();

        let mut holding = self.holding.lock().expect("nothing panics holding this");

        let standing = holding.get(&member.device).and_then(|held| {
            (held.expecting == expecting && held.presenting == presenting)
                .then(|| (held.client.clone(), Arc::clone(&held.met)))
        });

        if let Some((client, met)) = standing {
            met.lock().expect("nothing panics holding this").take();
            return Ok((client, met));
        }

        let met = Arc::new(Mutex::new(None));
        let client = self.relaying(expecting.clone(), &met)?;

        holding.insert(
            member.device.clone(),
            Held {
                expecting,
                presenting,
                met: Arc::clone(&met),
                client: client.clone(),
            },
        );

        Ok((client, met))
    }

    /// And the one a relayed call is made with: the same certificate presented
    /// and the same fingerprint pinned, and no deadline on the answer.
    ///
    /// **Because the work at the far end is the browser's rather than this
    /// device's.** [`ANSWERING`] is how long a machine has to say what it is,
    /// which is a reading it makes of itself; a relayed call is a press on a
    /// workbench, and the namespace it lands in has presses that cut a branch,
    /// make a worktree and push to GitHub. A deadline here would be this device
    /// deciding how long another one's work may take, and it would cut off
    /// exactly the calls worth relaying. What bounds the wait instead is the
    /// browser's own patience, which is the same thing that bounds a local
    /// press.
    ///
    /// [`REACHING`] stays, being about the address rather than the answer: a
    /// member that has moved costs a walk down its old addresses here as
    /// anywhere else.
    fn relaying(
        &self,
        expecting: Vec<String>,
        met: &Arc<Mutex<Option<String>>>,
    ) -> Result<reqwest::Client> {
        self.client(None, |algorithms| {
            Arc::new(ThePinnedOne {
                expecting,
                met: Arc::clone(met),
                algorithms,
            })
        })
    }

    /// And the one a join is made with: the same certificate presented, and
    /// whatever the far end shows accepted, with what it showed left in `met`.
    ///
    /// The one unpinned client in this module, for the reason [`Peers::join`]
    /// gives — there is nothing to pin it to until the humans have compared the
    /// fingerprints, and this call is how they come to have one to compare.
    fn asking(&self, met: &Arc<Mutex<Option<String>>>) -> Result<reqwest::Client> {
        self.client(Some(self.answering), |algorithms| {
            Arc::new(WhateverIsThere {
                met: Arc::clone(met),
                algorithms,
            })
        })
    }

    /// What all three of those come down to: this device's certificate to
    /// present, `verifier` deciding what the far end's has to be, a deadline on
    /// reaching a machine, and `answering` on being answered by one where there
    /// is one to keep.
    fn client(
        &self,
        answering: Option<Duration>,
        verifier: impl FnOnce(WebPkiSupportedAlgorithms) -> Arc<dyn ServerCertVerifier>,
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
            .with_custom_certificate_verifier(verifier(algorithms))
            .with_client_auth_cert(vec![certificate], key)
            .context("presenting this device's certificate to a peer")?;

        let built = reqwest::Client::builder()
            .use_preconfigured_tls(dialling)
            // The connection and the answer are two deadlines because they are
            // two different waits — see [`REACHING`] and [`ANSWERING`]. The
            // first is what a dead address costs, and it is spent once per
            // address; the second is what the machine that answered is given,
            // and it is spent once — and is not kept at all where what the far
            // end is answering is somebody else's request, which is
            // [`Peers::relaying`].
            .connect_timeout(self.reaching);

        match answering {
            Some(answering) => built.timeout(answering),
            None => built,
        }
        .build()
        .context("building the client a peer is dialled with")
    }
}

/// The certificates a dial to `member` will accept: the one recorded for it, and
/// the one it is changing over *from* where it is in the middle of a renewal.
///
/// **Two rather than one for exactly as long as a changeover lasts.** A device
/// that has re-issued its certificate announces the new fingerprint and then goes
/// on presenting the old one until the last of *its own* members has acknowledged
/// — which it has to, that being the only certificate every one of them holds —
/// and it tells nobody when that moment came. So a dial pinned on the fingerprint
/// this end recorded a second ago would be refused by the device that sent it, and
/// the renewal would cost every call it exists not to cost. Both are accepted, as
/// both get through this end's own gate; the one being changed from is let go of
/// when this device meets the new one, in [`Peers::took`].
///
/// The recorded one first, because it is the one a settled cluster has and the one
/// a changeover ends on: the order is what a reader of this list sees rather than
/// anything a handshake cares about.
fn accepted(member: &Member) -> Vec<String> {
    let mut accepted = vec![member.fingerprint.clone()];

    accepted.extend(member.renewing_from.clone());
    accepted
}

/// Where a call goes: `path` on the peer listener of whatever is at `address`.
///
/// One function for all three of them — the identity read, the join and the
/// cancel — because the *where* is one question however different the calls
/// are: an address somebody typed or a device advertised, on the port that
/// address names or on the peer port, over TLS because there is nothing else on
/// that listener.
fn reaching(address: &str, path: &str) -> String {
    format!("https://{}{path}", authority(address))
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
    /// The fingerprints recorded for this member, which are the only
    /// certificates that get through.
    ///
    /// **Usually one, and two while that member is in the middle of a changeover
    /// of its own.** A device that has re-issued its certificate goes on
    /// presenting the outgoing one until the last of *its* members has
    /// acknowledged the new one, and it tells nobody when that was — so a dial
    /// pinned on the fingerprint this end has just recorded would be refused by
    /// the very device that announced it. Both are accepted, exactly as both get
    /// through this end's own gate — see
    /// [`verkstead_store::Member::renewing_from`].
    ///
    /// Never empty: what would be pinned on nothing is a dial that accepts
    /// anything, and there is one call in this module that means to do that and
    /// it has a verifier of its own — see [`WhateverIsThere`].
    expecting: Vec<String>,

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

        if !self.expecting.iter().any(|expecting| expecting == &met) {
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

/// The other half of the pinning: whatever the far end presents is accepted,
/// and what it was is said.
///
/// **For the join and for nothing else.** A device pressing Add has never met
/// the machine it is about to ask, so there is no fingerprint to check against —
/// the fingerprint is what this call goes and *fetches*, for the two humans to
/// compare by eye and for every dial after this one to be pinned on. An
/// unpinned dial is safe here because nothing is decided by it: the post it
/// carries writes a question down at the far end, and a human pressing Allow is
/// what a link is made of.
///
/// The certificate met is left in [`WhateverIsThere::met`] because the caller
/// needs it: it is what is checked against the fingerprint the far end names of
/// itself, and what is written down as the device's own once the two agree.
#[derive(Debug)]
struct WhateverIsThere {
    /// Where the certificate that turned up is left, as its fingerprint.
    met: Arc<Mutex<Option<String>>>,

    /// The signature algorithms the provider supports, which is what the two
    /// signature checks below are run against.
    algorithms: WebPkiSupportedAlgorithms,
}

impl ServerCertVerifier for WhateverIsThere {
    /// Taken, whatever it is — and written down, which is the point of it.
    ///
    /// Nothing is asked about a host name, a chain or the validity dates. The
    /// name a dial goes out under is an address somebody typed; a chain has
    /// nowhere in a cluster to lead; and a certificate that has run out is
    /// refused at the far end's own listener, which is where a certificate
    /// presented to somebody is judged.
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        *self.met.lock().expect("nothing panics holding this") =
            Some(fingerprint_of_der(end_entity));

        Ok(ServerCertVerified::assertion())
    }

    /// And that the far end holds the key that signed what it presented, which
    /// is asked here exactly as it is of a pinned one: without it a certificate
    /// would be a public file anybody who had read one could replay, and the
    /// fingerprint the humans go on to compare would be somebody else's.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A device to dial *from*: an identity in a directory of its own, which is
    /// what a client presents.
    ///
    /// The directory is handed back with it because dropping one takes the
    /// certificate off the disk with it.
    fn dialling_as(id: &str) -> (Peers, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("a directory to keep an identity in");
        let device = Device::stated(dir.path(), id).expect("an identity to present");

        (Peers::of(device, Members::none()), dir)
    }

    /// And a member to dial: the Device Id and the fingerprints are the whole
    /// of what a client is built against, so everything else is the row's
    /// ordinary shape.
    fn a_member(device: &str, fingerprint: &str) -> Member {
        Member {
            device: device.to_owned(),
            name: "workbench".to_owned(),
            os: "Linux".to_owned(),
            addresses: vec!["192.168.1.24".to_owned()],
            fingerprint: fingerprint.to_owned(),
            last_seen: "2026-09-26T00:00:00Z".to_owned(),
            reachable: true,
            renewing_from: None,
            acknowledged: None,
        }
    }

    /// One client per member rather than one per call, so a remote
    /// Conversation's second read goes over the connection its first one
    /// opened.
    ///
    /// Asked of the handle the client's verifier writes into, that being the
    /// one thing made alongside a client and shared with it: the same handle
    /// back is the same client back.
    #[test]
    fn a_members_dialling_client_is_held_rather_than_built_for_every_call() {
        let (peers, _dir) = dialling_as("aa00bb11cc22dd33ee44ff5566778899");
        let member = a_member("0011223344556677889900aabbccddee", "sha256:theirs");

        let (_, first) = peers.held_for(&member).expect("a client to dial with");
        let (_, again) = peers.held_for(&member).expect("and the same one again");

        assert!(
            Arc::ptr_eq(&first, &again),
            "a second call to the one member should go over the client the first built",
        );
    }

    /// And a member apiece, because a client is pinned on the fingerprint it
    /// was built for.
    #[test]
    fn two_members_are_dialled_with_a_client_each() {
        let (peers, _dir) = dialling_as("aa00bb11cc22dd33ee44ff5566778899");

        let (_, one) = peers
            .held_for(&a_member("0011223344556677889900aabbccddee", "sha256:one"))
            .unwrap();
        let (_, other) = peers
            .held_for(&a_member("ffeeddccbbaa00998877665544332211", "sha256:two"))
            .unwrap();

        assert!(!Arc::ptr_eq(&one, &other));
    }

    /// A member that has re-issued its certificate is dialled with a client
    /// built for the new one: a held client pins the fingerprint it was made
    /// with, and one kept past a renewal would refuse the machine it is for.
    #[test]
    fn a_renewed_member_is_dialled_with_a_client_built_for_it() {
        let (peers, _dir) = dialling_as("aa00bb11cc22dd33ee44ff5566778899");
        let device = "0011223344556677889900aabbccddee";

        let (_, before) = peers.held_for(&a_member(device, "sha256:before")).unwrap();
        let (_, after) = peers.held_for(&a_member(device, "sha256:after")).unwrap();

        assert!(
            !Arc::ptr_eq(&before, &after),
            "the fingerprint moved, so the client pinned on the old one is not reused",
        );

        // And the changeover's second fingerprint counts the same way: a member
        // in the middle of one is accepted under two, and a client built for
        // one of them is not built for both.
        let mut renewing = a_member(device, "sha256:after");
        renewing.renewing_from = Some("sha256:before".to_owned());

        let (_, changing) = peers.held_for(&renewing).unwrap();

        assert!(!Arc::ptr_eq(&after, &changing));
    }

    /// And what a walk met is emptied before the next one, so that a mismatch
    /// from an earlier call is not handed back as the reason a member is not
    /// there now.
    ///
    /// The one thing a held client makes possible that a fresh one could not:
    /// the handle lives as long as the client does, and [`Peers::nowhere`]
    /// reports whatever is in it.
    #[test]
    fn what_an_earlier_walk_met_is_not_carried_into_the_next() {
        let (peers, _dir) = dialling_as("aa00bb11cc22dd33ee44ff5566778899");
        let member = a_member("0011223344556677889900aabbccddee", "sha256:theirs");

        let (_, met) = peers.held_for(&member).unwrap();

        *met.lock().unwrap() = Some("sha256:somebody-else".to_owned());

        let (_, next) = peers.held_for(&member).unwrap();

        assert_eq!(
            *next.lock().unwrap(),
            None,
            "the next walk starts having met nobody",
        );
    }

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

    /// And the address somebody types into **Add** is asked at the peer port
    /// where it names none, and at the port it names where it names one.
    ///
    /// Which is the whole of what a human should have to know about the port:
    /// every device answers on [`PEER_PORT`] unless its host was told another,
    /// and an install that was told another is reachable no other way. Asked of
    /// the join here because that is the call an address is *typed* for — a
    /// member's addresses were advertised by the machine they belong to.
    #[test]
    fn a_typed_address_is_asked_at_the_peer_port_or_at_the_one_it_names() {
        assert_eq!(
            reaching("workbench.tailnet-name.ts.net", JOIN),
            format!("https://workbench.tailnet-name.ts.net:{PEER_PORT}{JOIN}"),
        );
        assert_eq!(
            reaching("192.168.1.31", JOIN),
            format!("https://192.168.1.31:{PEER_PORT}{JOIN}"),
        );
        assert_eq!(
            reaching("192.168.1.31:9000", JOIN),
            format!("https://192.168.1.31:9000{JOIN}"),
        );
    }

    /// And a cancel goes back to the same address, under the request it is
    /// taking back.
    #[test]
    fn a_cancel_goes_back_to_the_address_the_join_was_asked_at() {
        assert_eq!(
            reaching("192.168.1.31:9000", &cancelling("1122334455667788")),
            "https://192.168.1.31:9000/api/peer/v1/join/1122334455667788/cancel",
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
