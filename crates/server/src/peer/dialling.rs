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
//! **With one exception, which is the join.** A device pressing Add has never
//! met the machine it is asking, so there is no fingerprint to pin it on: that
//! dial takes whatever certificate turns up, for the one call, and hands back
//! what it turned out to be — which is the string the two humans then compare by
//! eye and the string every dial after it is pinned on. See [`Peers::join`] and
//! [`WhateverIsThere`].
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
use verkstead_render::{DeviceIdentity, JoinHeld, JoinSettled};
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

        // A certificate met that was not the one recorded is a machine that
        // answered rather than a member that is not there — the same reading
        // [`Peers::identity`] makes of one, and the same refusal to dim a row
        // over it.
        if let Some(met) = met.lock().expect("nothing panics holding this").take() {
            bail!(
                "device {} is recorded against {}, and the machine answering for it presented \
                 {met} instead",
                member.device,
                member.fingerprint,
            );
        }

        self.members.unreachable(&member.device).await?;

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
    /// carries from here on.
    pub async fn join(&self, address: &str, saying: &DeviceIdentity) -> Result<JoinHeld> {
        let met = Arc::new(Mutex::new(None));
        let asking = self.asking(&met)?;
        let at = reaching(address, JOIN);

        let answered = asking
            .post(&at)
            .json(saying)
            .send()
            .await
            .with_context(|| format!("asking the device at {address} to link with this one"))?;

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
        self.client(|algorithms| {
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
        self.client(|algorithms| {
            Arc::new(WhateverIsThere {
                met: Arc::clone(met),
                algorithms,
            })
        })
    }

    /// What both of those come down to: this device's certificate to present,
    /// `verifier` deciding what the far end's has to be, and a deadline apiece
    /// on reaching a machine and on being answered by one.
    fn client(
        &self,
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
