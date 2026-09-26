//! What this Verkstead *is*: a device id and a self-signed certificate, both
//! invented at the first start and read back at every one after it (ADR-0020).
//!
//! Nothing on the wire said which install answered. Now each one names itself,
//! and the name is two things rather than one.
//!
//! **The id is what a record and a URL call a device by.** Random, and short
//! enough to sit in a URL segment. The tailnet node name and the hostname were
//! both rejected: the first is gone the moment the machine leaves the tailnet,
//! and the second collides — a Windows machine and its WSL answer to one
//! hostname, and a cluster that could not tell them apart would be a cluster
//! with one of them in it.
//!
//! **The certificate is the other half, and it is what a link is made of.** A
//! link between two devices is the two fingerprints each side holds, with no
//! bearer token and nothing stored beside the certificates — so the certificate
//! is not a detail of how a connection is encrypted but the thing being
//! identified. Here it is made, read back, named and made again before it runs
//! out; the peer listener that presents it is [`crate::peer`]'s.
//!
//! **A file of its own each, beside `workbench.key` rather than inside a
//! settings file** — see [`crate::key`], which is the same problem solved
//! once already and the shape this follows. A settings save writes the whole of
//! its file out of what the page was told, so an identity kept there would be
//! an identity somebody tidying up a token had replaced; and what is in those
//! files is what a human configured, where this is configured by nobody.
//! Written atomically at the same mode, whatever is already there read back,
//! and a fresh one written only where there is nothing.
//!
//! **And what a device is *like* is read rather than kept** — see [`reading`],
//! which is the other half of an identity: the hostname this machine answers
//! to, the word for the operating system it is running, and every address a
//! peer could reach it on. None of those is invented and none is written down,
//! because none of them is a fact about this install that outlives a request:
//! a laptop moves between the LAN and the tailnet, and DHCP moves everybody.
//!
//! **The validity is said here rather than taken from the crate, and the
//! certificate is re-issued before it runs out.** Ninety days, written down in
//! [`VALIDITY`], and made again at the first start with fewer than
//! [`RENEW_WITHIN`] of them left. An expired certificate is refused at the
//! handshake, so a certificate issued once and read back for ever would take
//! every link in a cluster down together on the same day, the only way back
//! being to re-link every device by hand — which is exactly what ADR-0020
//! ruled out, along with a validity long enough never to matter and a default
//! accepted without looking.
//!
//! **Both certificates are held over the changeover** — see [`Changeover`].
//! The fresh one waits beside the one being presented until every member has
//! acknowledged its fingerprint, so a re-issue never costs a call; with no
//! member to acknowledge anything the changeover completes at the start that
//! began it, and says it had nobody to tell. How many are owed is read off the
//! members this device keeps — see [`Members`] — and the telling that works that
//! list off one member at a time is [`Devices::announce_renewal`], made
//! presenting the outgoing certificate because that is the only one any member
//! holds. The last acknowledgement finishes the changeover, by the same two
//! writes the start that had nobody to tell makes — see
//! [`Device::changed_over`]. A member that was switched off is owed the telling
//! on the record a newcomer and an unlink are owed on, and is told by the next
//! dial that gets through to it — see [`Devices::caught_up`].
//!
//! **And [`Devices`] is what the human's own browser reads of all this**: this
//! device and every other device in its cluster, a row apiece, which is the
//! Devices section of the Remote access pane. This device's own row is the same
//! answer a stranger reads off the peer listener's identity endpoint, told to
//! the browser instead — that listener presents a certificate nothing but
//! another Verkstead has a reason to trust, so the workbench is where the pane
//! asks.

pub mod reading;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use rcgen::{CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, KeyPair};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use verkstead_render::{
    AskingDevice, DeviceIdentity, DevicesView, DiscoveredDevice, JoinHeld, JoinSettled,
    RenewedCertificate,
};
use verkstead_store::{AskedJoin, Linking, Telling};
use x509_parser::certificate::X509Certificate;
use x509_parser::prelude::FromDer;

use crate::discovery::{self, Browse, Probe};
use crate::peer::Members;
use crate::peer::dialling::{Peers, Reached};
use crate::peer::joining::Joins;
use crate::settings::write_atomically;

/// What the id's file is called inside the Data Directory, and what the
/// certificate's is. Fixed rather than configurable, for the reason the
/// database's name and the Workbench Key's are: the directory is what an
/// operator points Verkstead at, and what is in it is Verkstead's to name.
const ID_FILE: &str = "device.id";

/// The private key and the certificate together, in the order a PEM file
/// conventionally carries them. One file rather than two because they are one
/// thing: a certificate whose key is gone proves nothing, and a key whose
/// certificate is gone names nobody, so there is no state in which having one
/// of the two is better than having neither.
const CERTIFICATE_FILE: &str = "device.pem";

/// And where the certificate made to replace it waits, while any member has
/// still to be told its fingerprint — see [`Changeover`].
///
/// A file of its own rather than the first one being written over, because
/// [`CERTIFICATE_FILE`] is what this device *presents*. A changeover that
/// moved that file's meaning would be a start — one that lost power halfway,
/// or an older build's — presenting a certificate no member had acknowledged,
/// which is the call a changeover exists not to cost. So the new one waits
/// here, and the last thing a completed changeover does is write it over the
/// old one and take this away.
const INCOMING_FILE: &str = "device.next.pem";

/// And what all of them are written as: readable and writable by the account
/// Verkstead runs under, and by nothing else on the machine. The same mode the
/// Workbench Key's file gets, and for the same reason on the half that is a
/// private key — it is the whole of what proves this device is this device,
/// and a link is somebody else's machine trusting it.
///
/// The id is nobody's secret and is written at that mode anyway. It is one
/// identity in a handful of files, and two modes over them would be two things
/// to remember and one of them to get wrong.
const DEVICE_MODE: u32 = 0o600;

/// How much randomness the id is: sixteen bytes from the operating system's own
/// generator, which is the hundred and twenty-eight bits a UUID carries.
///
/// Not a secret — nothing is admitted by naming a device, the certificate being
/// what admits anything — so this is a width at which two devices never collide
/// rather than one at which neither can be guessed.
const ID_BYTES: usize = 16;

/// How long a certificate is good for: **ninety days**.
///
/// Said here rather than inherited. An expired certificate is refused at the
/// handshake, so this number is the day every link in a cluster would stop
/// working at once if nothing renewed the certificate — and a crate's default,
/// whatever it happens to be, is nobody's decision about that. Ninety days is
/// short enough that a renewal which has silently stopped happening is found
/// out about while there is still somebody around who remembers this feature,
/// and long enough that renewing is a quarterly event rather than a weekly one.
///
/// What keeps that day from arriving is [`RENEW_WITHIN`], which is how much of
/// this is left when the certificate is made again.
pub const VALIDITY: time::Duration = time::Duration::days(90);

/// And how much of that validity is left when it is: **thirty days**.
///
/// The first start inside this window makes a fresh certificate — see
/// [`Device::renewed`] — which leaves two months of ordinary starts to do it
/// in and a month of them after in which to notice that one did.
///
/// **At a start rather than on a timer**, because a start is when these files
/// are read at all, and a schedule inside the process would be a second thing
/// to get right about an event that every upgrade and every reboot already
/// walks into.
///
/// **Which leaves the server nobody restarts, and that is a trade rather than
/// an oversight.** A process running longer than [`VALIDITY`] without one goes
/// past this window and then past its own expiry still presenting the
/// certificate it started with, and every member refuses it at the handshake
/// until somebody restarts it. Sixty days of running before the window is so
/// much as reached is longer than any upgrade here has gone without a restart,
/// and the machine that manages it has a human on it who can restart it.
pub const RENEW_WITHIN: time::Duration = time::Duration::days(30);

/// What a start did about the expiry: whether a re-issue was due, and whether
/// anybody is owed an announcement of the certificate it made (ADR-0020).
///
/// Read off the handle rather than logged and forgotten, because it is where the
/// announcement is picked up: a changeover still owed one is a changeover with a
/// member to tell, and [`Devices::announce_renewal`] is what tells them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Changeover {
    /// Nothing was due: the certificate has more than [`RENEW_WITHIN`] left, so
    /// it is the one it was. Which is what all but one start in a
    /// certificate's life finds.
    NotDue,

    /// A fresh certificate was made, and it is the one presented from this
    /// moment on: nobody was owed an announcement of its fingerprint, so the
    /// changeover completed at the start that began it.
    ///
    /// Which is every re-issue on a Verkstead that has never been linked to
    /// anything: with no member recorded — see [`Members`] — the answer to *is
    /// anything still owed an announcement* is no, and saying so is the whole
    /// of what a changeover here has to do about it.
    NobodyToTell,

    /// A fresh certificate is held beside the one still being presented, and
    /// this many members have yet to acknowledge its fingerprint.
    ///
    /// The old one goes on going out until they have, so the changeover never
    /// costs a call: a member that was unreachable is announced to again when
    /// it next answers, and one that never answers is a member the human
    /// unlinks anyway.
    YetToTell(usize),
}

/// This device: its id, the certificate it presents, and the files they are
/// kept in.
///
/// A handle rather than the two strings, because the files are what the
/// identity *is*: a second Verkstead over the same Data Directory is the same
/// device, and a re-issue has somewhere to write.
#[derive(Debug, Clone)]
pub struct Device {
    /// The directory the files are in, which is the Data Directory. Kept
    /// rather than the paths, because each is the others' neighbour and a
    /// re-issue writes more than one of them.
    dir: PathBuf,

    id: String,

    /// The certificate this device presents, which over a changeover is the
    /// outgoing one: what a member holds is what a member is answered with.
    presenting: Held,

    /// And the one made to replace it, where a changeover is in flight. `None`
    /// is the ordinary state, and it is two different ordinary states — no
    /// re-issue is due, or the last one is over.
    incoming: Option<Held>,

    /// What this start did about the expiry, which is what the startup line
    /// says and what an announcement will be picked up from.
    changeover: Changeover,
}

/// A certificate as this module holds one: the text its file keeps, what it is
/// named by, and when it runs out.
///
/// The text is what is presented and what is read back, so it is kept rather
/// than reassembled out of parsed halves. The other two are worked out once as
/// it is read or made, there being no reading either of them without a parse.
#[derive(Debug, Clone)]
struct Held {
    certificate: String,

    /// Its fingerprint, which is compared by eye rather than by machine — two
    /// people on two phones checking that the device one of them is linking is
    /// the device the other is offering — so it is a string here rather than
    /// the digest it is a spelling of.
    fingerprint: String,

    /// And when it stops being accepted at a handshake, which is the one thing
    /// a start asks of a certificate it already has.
    expires: OffsetDateTime,
}

impl Device {
    /// The identity kept in `data_dir`: whatever is already there, and a fresh
    /// one written where there is nothing.
    ///
    /// **A second start reads the first's**, which is the whole point of
    /// keeping either on disk: an id invented at every start would be a device
    /// no record could name twice, and a certificate made at every start would
    /// be a link that stopped working when the machine rebooted.
    ///
    /// **One may be there without the other, and which one decides what
    /// happens.** A start that wrote the id and then failed to write the
    /// certificate leaves an id worth keeping, and the certificate is made out
    /// to that id at the next start rather than both being thrown away. The
    /// other way round there is nothing to keep: a certificate is made out to
    /// an id, so one standing beside an id that had to be invented is a
    /// certificate for a device that no longer exists, and a fresh id takes a
    /// fresh certificate with it.
    ///
    /// A file that is there and empty is treated as a file that is not there,
    /// for the reason the Workbench Key's is: nothing writes one, the writes
    /// below being atomic, so it is a machine that lost power or a hand that
    /// emptied it, and inventing is the only recovery either of those has.
    ///
    /// **A file that is there and cannot be read is a failure**, though, and so
    /// is a certificate file that is there and will not parse. That is where
    /// this parts company with the key, which takes any bytes at all: quietly
    /// issuing a fresh identity over the one a cluster has pinned is the single
    /// act here that cannot be taken back, so a file this start could not get
    /// at is one it stops over rather than writes past. A `device.pem` left
    /// owned by root by one start under `sudo` is exactly that file, and the
    /// directory around it stays writable — so the alternative is not a write
    /// that fails and reports itself but a rename straight over the certificate
    /// every linked device is holding. Either way the start says which file it
    /// was and what deleting it would cost, and stops — see [`read_back`] and
    /// [`Device::holding`].
    ///
    /// **And the expiry is seen to here**, which is the one thing a start does
    /// *to* an identity it found rather than with it — see [`Device::renewed`],
    /// which is where `members` is asked its one question.
    pub async fn issued(data_dir: &Path, members: &Members) -> std::io::Result<Device> {
        let id_path = data_dir.join(ID_FILE);
        let certificate_path = data_dir.join(CERTIFICATE_FILE);

        // The id first, and whether it was already there — because the
        // certificate follows from it. A certificate is made out to an id, so
        // one found beside an id that had to be invented is a certificate for a
        // device that no longer exists, and a fresh id takes a fresh
        // certificate with it.
        let (id, kept) = match read_back(&id_path)? {
            Some(id) => (id, true),
            None => {
                let id = invented()?;

                write_atomically(&id_path, &format!("{id}\n"), DEVICE_MODE)?;

                (id, false)
            }
        };

        // And then the certificate: the one on disk where the id it was made
        // out to is the id above, and a fresh one everywhere else.
        let certificate = match kept {
            true => read_back(&certificate_path)?,
            false => None,
        };

        let certificate = match certificate {
            Some(certificate) => certificate,
            None => {
                let certificate = minted(&id, VALIDITY)?;

                write_atomically(&certificate_path, &certificate, DEVICE_MODE)?;

                certificate
            }
        };

        Device::holding(data_dir, id, certificate)?
            .renewed(members)
            .await
    }

    /// The identity a fixture states, so that what a suite asserts against is
    /// an id it chose rather than sixteen random bytes it has to filter out of
    /// a payload.
    ///
    /// The same files in the same place, with the id written rather than
    /// invented — an id is an id whatever made it, and a suite that had to
    /// pattern-match one would be a suite pinning everything but the field it
    /// is about. See [`crate::key::WorkbenchKey::stated`], which is here for
    /// that reason.
    ///
    /// **The certificate is made rather than stated**, and that is deliberate:
    /// a golden certificate carries a fixed expiry, so a fixture holding one
    /// would be a suite that began failing on a date nobody chose. What a test
    /// needs of the fingerprint it reads off the handle this hands back, which
    /// is the same string the server would present.
    pub fn stated(data_dir: &Path, id: &str) -> std::io::Result<Device> {
        Device::stated_good_for(data_dir, id, VALIDITY)
    }

    /// And the same with how much life is left in the certificate stated too,
    /// which is what the suite about the renewal stands on.
    ///
    /// A start near the expiry is the whole of what that suite has to be able
    /// to stand at, and there are two ways of standing there: a clock this
    /// process does not keep, or a certificate that was made with less life in
    /// it. This is the second. The certificate is a real one made the way
    /// every other one here is made, and the only thing stated about it is how
    /// long it was ever good for — so what [`Device::issued`] then does with it
    /// is what it would do on a machine that had been running for two months.
    ///
    /// It writes the identity and hands back what was written. The re-issue is
    /// [`Device::issued`]'s, as it is at a start.
    pub fn stated_good_for(
        data_dir: &Path,
        id: &str,
        good_for: time::Duration,
    ) -> std::io::Result<Device> {
        write_atomically(&data_dir.join(ID_FILE), &format!("{id}\n"), DEVICE_MODE)?;

        let certificate = minted(id, good_for)?;

        write_atomically(&data_dir.join(CERTIFICATE_FILE), &certificate, DEVICE_MODE)?;

        Device::holding(data_dir, id.to_owned(), certificate)
    }

    /// One handle over an id and a certificate that are both already on disk,
    /// with nothing said yet about the expiry.
    ///
    /// Where the certificate is parsed, which is what turns text into an
    /// identity — see [`held`]. A failure here is a failure at the start that
    /// read it, saying which file it was and what deleting it would cost.
    fn holding(data_dir: &Path, id: String, certificate: String) -> std::io::Result<Device> {
        let presenting = held(&certificate).map_err(|why| {
            std::io::Error::other(format!(
                "reading the device certificate in {}: {why:#} — delete {CERTIFICATE_FILE} and \
                 the next start makes a fresh one, which every device this one is linked to \
                 would then have to be linked to again",
                data_dir.display(),
            ))
        })?;

        Ok(Device {
            dir: data_dir.to_owned(),
            id,
            presenting,
            incoming: None,
            changeover: Changeover::NotDue,
        })
    }

    /// The same device with the expiry seen to: a fresh certificate where one
    /// is due, and the changeover already in flight where an earlier start
    /// began one.
    ///
    /// **`members` is asked one question** — how many of them have yet to
    /// acknowledge the new fingerprint — and its answer decides which of the
    /// two certificates is presented. Nobody owed an announcement is a
    /// changeover that completes here and now; anybody owed one is a
    /// changeover in flight, the old certificate still going out and the new
    /// one waiting in [`INCOMING_FILE`] for the start after this. That is the
    /// whole of the bookkeeping. Telling them is still to come, so what a
    /// member is owed is never worked off and every member there is is owed —
    /// which on a Verkstead that has never been linked to anything is nobody,
    /// and [`Changeover::NobodyToTell`] is it saying so.
    async fn renewed(self, members: &Members) -> std::io::Result<Device> {
        // A changeover an earlier start began, where there is one. The
        // certificate it made is read back rather than a third one minted:
        // every restart during a changeover would otherwise be another
        // fingerprint for the members to acknowledge, and a changeover that
        // never finished.
        let begun = match read_back(&self.incoming_path())? {
            Some(pem) => Some(held(&pem).map_err(|why| {
                std::io::Error::other(format!(
                    "reading the certificate this device is changing over to, in {}: {why:#} — \
                     delete {INCOMING_FILE} and the next start begins the changeover again, \
                     which costs nothing while nothing has acknowledged it",
                    self.dir.display(),
                ))
            })?),
            None => None,
        };

        // Or one begun here, where the expiry is near enough for that.
        let incoming = match begun {
            Some(incoming) => Some(incoming),
            None if self.due() => {
                let certificate = minted(&self.id, VALIDITY)?;

                Some(held(&certificate).map_err(std::io::Error::other)?)
            }
            None => None,
        };

        let Some(incoming) = incoming else {
            return Ok(self);
        };

        match members
            .unacknowledged(&incoming.fingerprint)
            .await
            .map_err(|why| std::io::Error::other(format!("{why:#}")))?
        {
            // Nobody is owed an announcement, so the changeover completes at
            // once — by the same two writes the last acknowledgement makes, this
            // being one path rather than two things that have to stay in step:
            // see [`Device::changed_over`].
            0 => {
                self.changed_over(&incoming)?;

                Ok(Device {
                    presenting: incoming,
                    incoming: None,
                    changeover: Changeover::NobodyToTell,
                    ..self
                })
            }

            // And somebody is, so the old certificate goes on being presented
            // and the new one waits where the start after this will find it.
            // Written again although it may have just been read from there:
            // one write of the same bytes at each start while a changeover is
            // in flight is cheaper than two paths through here that have to
            // stay in step.
            owed => {
                write_atomically(&self.incoming_path(), &incoming.certificate, DEVICE_MODE)?;

                Ok(Device {
                    incoming: Some(incoming),
                    changeover: Changeover::YetToTell(owed),
                    ..self
                })
            }
        }
    }

    /// The changeover finished, on disk: the certificate that was waiting becomes
    /// the one this Data Directory holds, and the file it was waiting in goes.
    ///
    /// **In that order**, so that a machine losing power between the two leaves a
    /// start holding two copies of one certificate rather than none — which is a
    /// changeover that completes again and comes out where this one did. Taking
    /// the file away counts one that is already gone as taken away, for the same
    /// reason.
    ///
    /// **Two callers and one path.** A re-issue with nobody to tell finishes at
    /// the start that began it — see [`Device::renewed`] — and one with members
    /// finishes at the last acknowledgement, in
    /// [`Devices::changeover_settled`]. They are the same two writes, so they are
    /// the same function: a second copy of them beside this one would be two
    /// changeovers that ended in subtly different states.
    ///
    /// **What it does *not* do is change what this process presents.** The
    /// certificate a running listener stands behind was built into its TLS
    /// configuration at the start, and the handle every dial is made with is a
    /// clone of this one — so a changeover that completes under a running server
    /// goes on presenting the outgoing certificate until the next start, which is
    /// where this file is read. That costs nothing: every member holds both by
    /// then, having acknowledged the new one and kept the old beside it precisely
    /// against this — see [`verkstead_store::Member::renewing_from`] — and it is
    /// what keeps a completing changeover from having to rebuild a listener and a
    /// client under live traffic.
    fn changed_over(&self, incoming: &Held) -> std::io::Result<()> {
        write_atomically(&self.certificate_path(), &incoming.certificate, DEVICE_MODE)?;

        taken_away(&self.incoming_path())
    }

    /// The same, for the changeover that finishes while this process is running:
    /// the last member has acknowledged, so the certificate waiting in
    /// [`INCOMING_FILE`] is the one this Data Directory holds from now on.
    ///
    /// Nothing at all where no changeover is in flight, which is a caller that
    /// counted nobody owed on a device that never re-issued anything.
    pub(crate) fn changeover_complete(&self) -> std::io::Result<()> {
        match &self.incoming {
            Some(incoming) => self.changed_over(incoming),
            None => Ok(()),
        }
    }

    /// Whether the certificate being presented is near enough its expiry to be
    /// made again: [`RENEW_WITHIN`] or less left of it.
    ///
    /// One that has already run out is near enough too, which is what a
    /// machine switched off for a season comes back to. There is nothing
    /// better to do with an expired certificate than replace it — every member
    /// refuses it at the handshake, so the links it was holding are down
    /// already and the changeover costs nothing that is not lost.
    fn due(&self) -> bool {
        self.presenting.expires - OffsetDateTime::now_utc() <= RENEW_WITHIN
    }

    /// What every record and URL names this device by.
    ///
    /// Untouched by a re-issue: it is the certificate that is renewed, and a
    /// device keeps its id for as long as its Data Directory lasts.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The certificate and its private key, as the file holds them: the PEM a
    /// TLS configuration is built out of.
    ///
    /// Over a changeover this is the *outgoing* one, which is the point of
    /// holding two: what the members hold is what they are answered with,
    /// until they have said they hold the other.
    pub fn certificate(&self) -> &str {
        &self.presenting.certificate
    }

    /// The certificate's fingerprint, in the spelling a human compares one in:
    /// the SHA-256 of its bytes as upper-case hex in colon-separated pairs,
    /// which is what every tool that has ever printed one prints.
    ///
    /// That spelling is the point of it. The fingerprint is what one person
    /// reads off a phone while another reads it off a screen, so it is grouped
    /// to be found a place in and cased to be read aloud.
    pub fn fingerprint(&self) -> &str {
        &self.presenting.fingerprint
    }

    /// And the fingerprint of the certificate waiting to replace it, where a
    /// changeover is in flight.
    ///
    /// Both are printable over a changeover because that is the only way
    /// anybody tells which of the two a peer met: a human reading a startup
    /// line off one machine, and a member's record of what it has acknowledged,
    /// are looking at the same pair of strings.
    pub fn incoming_fingerprint(&self) -> Option<&str> {
        self.incoming
            .as_ref()
            .map(|incoming| incoming.fingerprint.as_str())
    }

    /// What the start that read this identity did about the expiry.
    pub fn changeover(&self) -> Changeover {
        self.changeover
    }

    /// Where the id is kept, which is what says so in a failure to write it.
    pub fn id_path(&self) -> PathBuf {
        self.dir.join(ID_FILE)
    }

    /// And where the certificate is.
    pub fn certificate_path(&self) -> PathBuf {
        self.dir.join(CERTIFICATE_FILE)
    }

    /// And where the one replacing it waits, over a changeover.
    pub fn incoming_path(&self) -> PathBuf {
        self.dir.join(INCOMING_FILE)
    }
}

/// Why a call was not put to a member — see [`Devices::relay`], and
/// [`crate::relaying`], which is what turns each of these into an answer the
/// browser reads.
///
/// **Four findings rather than one refusal**, because they are four different
/// things to have found out and the page does different things with them: two
/// are a Device Id that was never going to be dialled, one is a machine that is
/// not there, and one is this device's own store. What they have in common is
/// that none of them is the far end's answer, which is the only other thing a
/// relayed call ever comes back as.
pub(crate) enum Unrelayed {
    /// The id named is this device's own. Local URLs keep their shape, so
    /// nothing should ever ask.
    ThisDevice,

    /// And the id named is no member's: this device holds no certificate for
    /// it and no address to reach it at.
    NoSuchMember,

    /// The member is one of this device's and answered at none of the
    /// addresses it advertised — or the machine that answered was not it.
    Unreachable(anyhow::Error),

    /// And the membership itself could not be read, which is this device's own
    /// trouble rather than anything about the device named.
    Unreadable(anyhow::Error),
}

/// What the **Devices** section of the Remote access pane reads: this device,
/// and every other device in its cluster (ADR-0020).
///
/// **The browser's side of the identity endpoint.** A peer reads what this
/// device is off [`crate::peer::IDENTITY`], over TLS on a port of its own; the
/// human's browser cannot — that listener presents a certificate nothing but
/// another Verkstead has any reason to trust, and the workbench is where the
/// pane is drawn. So the same answer is assembled again over here, out of the
/// same two handles the peer listener assembles it out of, rather than the
/// workbench dialling its own peer port to ask itself who it is.
///
/// **Held rather than read, and read at the moment it is asked.** The device is
/// the id and the certificate off the disk, which do not change under a running
/// server; what a device is *like* — the hostname, the OS and the addresses —
/// is read per answer, for the reason [`reading`] gives: a laptop moves between
/// the LAN and the tailnet, and DHCP moves everybody.
#[derive(Debug, Clone)]
pub struct Devices {
    /// What this device is: the id every record names it by and the certificate
    /// a link is made of.
    device: Device,

    /// And the machine it is on, as a way of asking rather than as an answer —
    /// see [`reading::Reading`].
    reading: reading::Reading,

    /// And who it is linked to, which is a row apiece on the list and the count
    /// on the card — the count coming off the rows rather than being answered
    /// beside them, there being one membership and one answer about it. Rows in
    /// the store, read at the moment the pane asks — see [`crate::peer::Members`].
    members: Members,

    /// And who it has *asked* to be linked to and not yet been answered by,
    /// which is a row apiece under those — see [`crate::peer::joining::Joins`].
    joins: Joins,

    /// And what it has heard of the devices it is *not* linked to, which is the
    /// Discovered list under those rows — see [`crate::discovery::Browse`].
    ///
    /// Here rather than beside this handle, because the three things a discovered
    /// row is left out for are all in this one: the id of this device, the
    /// membership, and the joins it is waiting on. A browse hears a LAN and knows
    /// none of them.
    ///
    /// Heard nothing unless a server said otherwise — see [`Devices::browsing`],
    /// which is where a start hands over the real one.
    browse: Browse,

    /// And the other half of that list, which is the tailnet: the peers
    /// `tailscale status` names, asked what they are as the list is read — see
    /// [`crate::discovery::Probe`].
    ///
    /// Beside the browse rather than inside it, the two being different kinds of
    /// finding: one is a multicast held open while somebody is looking, and this
    /// is a handful of dials made at the moment of asking. What they share is
    /// where they are merged and where the three exclusions are made, which is
    /// this handle.
    ///
    /// Asked nothing unless a server said otherwise — see [`Devices::probing`].
    probe: Probe,

    /// And how it reaches another device, which is what the one press in this
    /// section goes out over: Add dials the address somebody typed and posts a
    /// join, and Cancel dials the same address and takes it back.
    ///
    /// Built here out of the two handles above rather than passed in, because
    /// there is nothing to choose: what a dial presents is this device's own
    /// certificate and what it writes into is this device's own membership, and
    /// both are already in hand.
    peers: Peers,
}

impl Devices {
    /// The list a server answers out of: what it is, the machine it is on, its
    /// membership, and the joins it is waiting on.
    ///
    /// The same four the peer listener is built from, because they are the same
    /// four things — what is different is who is asking.
    pub fn of(
        device: Device,
        reading: reading::Reading,
        members: Members,
        joins: Joins,
    ) -> Devices {
        let peers = Peers::of(device.clone(), members.clone());

        Devices {
            device,
            reading,
            members,
            joins,
            browse: Browse::heard_nothing(),
            probe: Probe::asked_nothing(),
            peers,
        }
    }

    /// The same, browsing the LAN for the devices this one is not linked to — see
    /// [`crate::discovery::Browse`], which is where the Discovered list comes
    /// from.
    ///
    /// **Handed over rather than made here**, because a browse is a thread of its
    /// own and a multicast group: a router stood up to answer a question about a
    /// membership has no business joining one, so what [`Devices::of`] leaves is a
    /// device that has heard nothing and the one start that serves a workbench is
    /// what says otherwise.
    pub fn browsing(self, browse: Browse) -> Devices {
        Devices { browse, ..self }
    }

    /// The same, asking the nodes of this machine's tailnet what they are as that
    /// list is read — see [`crate::discovery::Probe`], which is the other half of
    /// where the Discovered list comes from.
    ///
    /// **Handed over rather than made here** for the reason the browse is, and
    /// one more besides: a probe is a handful of outbound handshakes at machines
    /// on somebody's tailnet, and a router stood up to answer a question about a
    /// membership has no business making them.
    pub fn probing(self, probe: Probe) -> Devices {
        Devices { probe, ..self }
    }

    /// The same, giving every dial this section makes `patience` rather than the
    /// deadlines a running server keeps — see [`Peers::waiting`].
    ///
    /// For a suite standing in front of an address nothing answers at: what is
    /// being asked there is what the press says when nobody is home, and a test
    /// that waited out the real deadline would be spending its time on the clock
    /// rather than on the question.
    pub fn waiting(self, patience: std::time::Duration) -> Devices {
        Devices {
            peers: self.peers.waiting(patience),
            ..self
        }
    }

    /// The list as the pane draws it, assembled now.
    ///
    /// `async` for the reason the identity endpoint's handler is: the tailnet
    /// half of the addresses is a command run on this machine, and a pane opened
    /// a moment later would get a different and equally true answer. And the
    /// members are read now for the same reason again — a join or an unlink on
    /// another tab is a different and equally true answer too.
    ///
    /// Fallible where the row for this device is not, because the membership is
    /// a database and the identity is two files already in hand. A pane that
    /// drew this device alone when the members could not be read would be a
    /// cluster that looked dissolved.
    pub(crate) async fn listing(&self) -> Result<DevicesView> {
        Ok(DevicesView {
            this: self.reading.identity(&self.device).await,
            members: self.members.listed().await?,
            pending: self.joins.pending(OffsetDateTime::now_utc()).await?,
        })
    }

    /// The **Discovered** list as the pane draws it: every device this one has
    /// heard of and is not already in a cluster with (ADR-0020, *Discovery*).
    ///
    /// **A reading of its own rather than a field of [`Devices::listing`]**, and
    /// that is the point of it: a browse hears something every few seconds, and a
    /// list that arrived on the same answer as the membership would be the rows
    /// the pane had already drawn replaced each time the LAN said anything. Two
    /// readings are two queries, and what a found device re-reads is this one.
    ///
    /// **Three kinds of device are left out, and this is where.** A **Member** is
    /// in the cluster already, so a row offering to link it would be a press with
    /// nothing behind it. This device hears its own advertisement, and a Verkstead
    /// is not linked to itself. And a device this one holds a **Join** for is one
    /// somebody has already pressed Add on — the pending row under the list is
    /// the answer to that press, and a discovered row beside it would be a second
    /// thing to press about one device. A refused or run-out row counts: it is
    /// still drawn up there, and what ends it is the Dismiss on it.
    ///
    /// **The exclusions are made here rather than by the browse or the probe**,
    /// because this is where a membership is known: a browse hears a LAN and a
    /// probe asks a tailnet, and neither has any idea which of what it found is
    /// already linked. So what they hold is everything they heard — which is also
    /// why hearing a member costs an announcement the next read draws nothing new
    /// from.
    ///
    /// **Two halves, merged by Device Id before any of that** — see
    /// [`crate::discovery::merged`]. A machine on the same network as this one and
    /// on the same tailnet is found twice and is one row saying both, and it is
    /// excluded or drawn as one thing.
    pub(crate) async fn discovered(&self) -> Result<Vec<DiscoveredDevice>> {
        let members: Vec<String> = self
            .members
            .rows()
            .await?
            .into_iter()
            .map(|member| member.device)
            .collect();

        let awaiting = self.joins.awaiting().await?;

        // The LAN's rows are what a browse already holds and the tailnet's are a
        // handful of dials made now — see [`crate::discovery::Probe`], which is
        // read at the moment of asking because a tailnet has nothing to announce
        // itself with.
        let heard = self.browse.found();
        let asked = self.probe.found(&self.peers).await;

        Ok(discovery::merged(heard, asked)
            .into_iter()
            .filter(|row| {
                row.device != self.device.id()
                    && !members.contains(&row.device)
                    && !awaiting.contains(&row.device)
            })
            .collect())
    }

    /// **Add**: ask the device at `address` to let this one into its cluster.
    ///
    /// The one place on the Remote access pane where something is configured
    /// rather than read — which is the departure Unlink makes beside it, and the
    /// one Remove on a Repo made before either.
    ///
    /// **Three things happen and the third is what is left behind.** This device
    /// says what it is; the far end writes that down as a question for its own
    /// human and answers with what *it* is; and this device writes a pending row
    /// naming the request, so that Cancel has something to name and the pane has
    /// something to draw. Nothing has been agreed: no member is recorded here and
    /// none is recorded there, and what settles it is a press on the other
    /// machine.
    ///
    /// **A device cannot ask itself**, which is settled in [`Devices::asked`]:
    /// the pane shows this machine's own addresses a few lines above the box, so
    /// typing one in is an easy mistake and a confusing state to be left in.
    pub(crate) async fn add(&self, address: &str) -> Result<()> {
        let address = address.trim();

        if address.is_empty() {
            bail!("an address to ask at is the one thing Add takes");
        }

        let saying = self.reading.identity(&self.device).await;
        let held = self.peers.join(address, &saying).await?;

        self.asked(address, held).await
    }

    /// **Add** on a **Discovered** row: ask the device that row is about, with
    /// nothing typed (ADR-0020, *Discovery*).
    ///
    /// **Named by its Device Id rather than by an address**, because a discovery
    /// found a *list* of them: mDNS resolves every address a device advertised,
    /// and the identity a probe read carries the tailnet's. So what is dialled is
    /// the list the row holds, in the order it was found — the LAN's first, that
    /// being the shorter road — exactly as a dial to a **Member** works down that
    /// member's addresses. The typed box keeps the single address it has always
    /// had: what somebody types is only the first address ever known.
    ///
    /// **What it leaves is what the typed press leaves**: a join posted, the
    /// question held for its ten minutes over there, and a pending row here with
    /// this device's own fingerprint under it and a Cancel. The discovered row
    /// goes with the press, a device a join is pending for being one the list
    /// leaves out — so a press moves a row from one list to the other rather than
    /// leaving two rows about one device.
    ///
    /// **And a row can be stale by the time it is pressed.** The device may have
    /// gone off the LAN or left the tailnet between the browse hearing it and
    /// somebody pressing Add, so the press is refused in the words a dial that
    /// reached nobody uses, naming the device — and the row is forgotten, which is
    /// what makes the next read of the list one without it.
    ///
    /// **A device that answered and said no keeps its row**, which is the one
    /// thing the forget above must not reach. A far end already holding as many
    /// join requests as it will, or one that could not write the question down,
    /// is a machine that is exactly where the row said it was — and forgetting it
    /// would take away the row somebody would press again in a minute. Only a
    /// walk that reached *nobody* says the row was wrong, which is what
    /// [`Reached`] is for.
    pub(crate) async fn add_found(&self, device: &str) -> Result<()> {
        let device = device.trim();

        // Off the list this device draws rather than off the browse, because the
        // three exclusions are the list's: a press naming a member, this device or
        // a device already being waited on is a press on a row nobody was offered.
        // Which costs the tailnet half a second asking again, and is what it
        // costs: a device found over the tailnet holds no address the browse has,
        // and a press is a dial either way.
        let Some(found) = self
            .discovered()
            .await?
            .into_iter()
            .find(|row| row.device == device)
        else {
            bail!(
                "device {device} is not one this device has heard of, so there is no address \
                 to dial it at",
            );
        };

        let saying = self.reading.identity(&self.device).await;

        let (address, held) = match self.peers.join_found(&found, &saying).await {
            // A device that spoke is a device that is there, whether what it said
            // was a question held or a refusal: the row stays exactly as it was,
            // and the press is refused in the far end's own terms.
            Reached::Answered(answered) => answered?,

            Reached::Nobody(why) => {
                // Nothing answered anywhere, so the row was wrong and this press
                // is where that was learned: forgotten here rather than left for a
                // TTL, so that the answer this refusal is drawn beside is a list
                // without it.
                self.browse.forgotten(&found.device);

                return Err(why);
            }
        };

        self.asked(&address, held).await
    }

    /// The pending row both presses leave behind: the request named, the address
    /// it was asked at, and the certificate this device met at the far end.
    ///
    /// **Shared because the two presses are one act at two starting points** —
    /// an address somebody typed, and a row a discovery drew. What differs is
    /// which address was dialled, and by here that is settled: it is the one that
    /// answered.
    ///
    /// **And a device cannot ask itself.** The pane shows this machine's own
    /// addresses a few lines above the box, so typing one in is an easy mistake
    /// and a confusing state to be left in — a modal on this workbench asking
    /// whether to link to this workbench. It cannot be told before the dial,
    /// there being nothing to compare until the far end has answered, so what is
    /// done is to take the question straight back off the machine that turned out
    /// to be this one. Nothing a *discovered* row leads to can be this device —
    /// the list leaves its own id out — so this is the typed box's case, kept
    /// here because it is a fact about what answered rather than about which
    /// press asked.
    async fn asked(&self, address: &str, held: JoinHeld) -> Result<()> {
        if held.identity.device == self.device.id() {
            // Taken back rather than left to run out, because the question is
            // this device's own and it is standing in front of its own human.
            // A cancel that cannot get through changes nothing: the request runs
            // out on its own, which is what it was going to do anyway.
            if let Err(why) = self
                .peers
                .cancel(address, &held.request, &held.identity.fingerprint)
                .await
            {
                tracing::info!(%why, "a request this device made to itself could not be taken back");
            }

            bail!("{address} is this device, and a device is not linked to itself");
        }

        self.joins
            .ask(&AskedJoin {
                request: held.request,
                address: address.to_owned(),
                device: held.identity.device,
                name: held.identity.name,

                // The fingerprint met at the far end, which the answer has just
                // been checked against — see [`Peers::join`]. What it is *for*
                // is the dial back that answers an Allow: the machine that dials
                // this one has to turn out to be the machine this one met.
                fingerprint: held.identity.fingerprint,

                asked_at: OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .context("saying when this device asked to link")?,

                // The far end's own word for when it lets go, rather than this
                // device's reckoning — see [`verkstead_render::JoinHeld`].
                expires_at: held.expires,

                // And nobody has said no, this being the moment the question
                // was asked: what writes that is the dial back the far end
                // makes if its human presses Deny.
                refused: false,
            })
            .await
    }

    /// **Cancel** on a pending row, and **Dismiss** on one that has run out:
    /// take the request back.
    ///
    /// **One press rather than two**, because the two are the same act seen at
    /// two moments — the human is done with a request that has not been answered
    /// — and which of them it is is a fact about the row rather than a choice.
    /// What differs is only whether there is anything at the far end left to
    /// tell: a request that has run out is one the other device has already let
    /// go of, so nothing is dialled for it.
    ///
    /// **A second press is not a second thing happening.** A row this device is
    /// not waiting on is one it has already stopped waiting on, and saying so
    /// twice is not a failure — the stance [`verkstead_store::forget_member`]
    /// takes, for its reason.
    ///
    /// **And a far end that cannot be reached does not keep the row.** The human
    /// has said they are done with it; a row that would not go away because
    /// somebody's laptop is shut would be the press not working. What is left
    /// over there runs out inside the ten minutes on its own.
    pub(crate) async fn take_back(&self, request: &str) -> Result<()> {
        let Some(asked) = self.joins.asked(request).await? else {
            return Ok(());
        };

        if !crate::peer::joining::run_out(&asked.expires_at, OffsetDateTime::now_utc())
            && let Err(why) = self
                .peers
                .cancel(&asked.address, &asked.request, &asked.fingerprint)
                .await
        {
            tracing::info!(
                %why,
                request = %asked.request,
                "a cancelled request could not be taken off the device it was asked of, \
                 which lets go of it when its ten minutes run out",
            );
        }

        self.joins.forget(request).await
    }

    /// Every device asking to be let into this one's cluster, as the modal in
    /// every open workbench draws it.
    ///
    /// The other side of the pending rows [`Devices::listing`] carries, and a
    /// reading of its own rather than a field of that one: the modal is raised
    /// wherever the human happens to be looking, so it is drawn in the shell
    /// every page sits inside and reads this on its own — a question that only
    /// arrived when somebody had the settings open would be a question the
    /// human never saw.
    pub(crate) async fn asking(&self) -> Result<Vec<AskingDevice>> {
        self.joins.asking(OffsetDateTime::now_utc()).await
    }

    /// **Allow**: let the device that asked into this one's cluster.
    ///
    /// **The membership's first real row, and the call that makes it a link.**
    /// Four things happen: the asker is recorded as a member out of what it
    /// said about itself in the join post, the request is let go of, the asker
    /// is dialled back and handed this device and every member it holds — see
    /// [`Peers::settle`] — and then every one of those members is told about
    /// the asker. The first three are in that order because the order is what a
    /// failure between them decides: a member recorded with the request still
    /// held is an Allow the human can press again, and a request let go of with
    /// no member written is a join that has to be made from the beginning.
    ///
    /// **The fourth is what joins the newcomer to everybody rather than to this
    /// device.** One press is the cluster's only gate, so the vouching has to
    /// be carried by something — and what carries it is the link this device
    /// already holds to each member: the announcement arrives there over a
    /// certificate that member has verified, and nobody over there is asked to
    /// confirm anything. A newcomer that introduced itself would be a stranger
    /// asking to be recorded, which is the arrangement ADR-0020 turned down.
    /// See [`Devices::announce`], where a member that answers nothing is dimmed
    /// and owed the telling rather than failing this press.
    ///
    /// **And a dial back that cannot be made does not undo the press.** The
    /// human pressed Allow and the asker is a member; a laptop that was shut
    /// between the question and the answer is that machine's problem rather
    /// than this press's, and what is left over there is a row that runs out
    /// and an Add to press again. So the failure is a line in this machine's
    /// log, which is where a peer that could not be reached is said.
    ///
    /// **The roster rather than this device alone.** Every member goes over in
    /// the one call, so that a newcomer joining a cluster of three lands
    /// holding all three — in a cluster of two the list is empty, and it is
    /// carried all the same, the handover being one shape whatever the
    /// cluster's size.
    ///
    /// **A second press is not a second thing happening**, which is the whole
    /// of what two workbenches showing one modal need: the first settles the
    /// request, and the second finds nothing held and does nothing — rather
    /// than a second member landing or an error being shown for having lost a
    /// race. A request whose ten minutes ran out is not held either, so an
    /// expiry settles it in exactly the same words.
    pub(crate) async fn allow(&self, request: &str) -> Result<()> {
        let Some(held) = self.joins.held(request, OffsetDateTime::now_utc()).await? else {
            return Ok(());
        };

        // What goes over, read before the asker is written down so that the
        // roster is the cluster as it was asked to be joined: this device as it
        // answers for itself, and everybody it was already linked to.
        //
        // The rows rather than the identities alone, because the same reading
        // is both halves of what an Allow does: it is the roster the newcomer
        // is handed, and it is the list of devices the newcomer is announced to
        // — and a member written down between the two would be announced to
        // about itself.
        let introducer = self.reading.identity(&self.device).await;
        let already = self.members.rows().await?;
        let members = already.iter().map(presenting).collect();

        self.members
            .refreshed(&Linking {
                device: held.device.clone(),
                name: held.name.clone(),
                os: held.os.clone(),

                // Every address it advertised, in the order it advertised them,
                // which is the order a dial to it will work down — see
                // [`crate::peer::dialling`].
                addresses: held.addresses.clone(),

                // And the certificate the handshake took from it, which is the
                // whole of what will prove it at this device's gate from now
                // on: a member *is* a fingerprint.
                fingerprint: held.fingerprint.clone(),
            })
            .await?;

        self.joins.let_go(request).await?;

        tracing::info!(
            device = %held.device,
            name = %held.name,
            request = %request,
            "a device has been let into this one's cluster",
        );

        if let Err(why) = self
            .peers
            .settle(
                &held,
                &JoinSettled::Joined {
                    introducer,
                    members,
                },
            )
            .await
        {
            tracing::info!(
                %why,
                device = %held.device,
                request = %request,
                "a device that was let in could not be told, so it is a member here and \
                 is still waiting over there until its own request runs out",
            );
        }

        // And now the others. One press joins the newcomer to everybody, and
        // this is what carries it: each member is told over the link this
        // device already holds to it, and records the newcomer without anybody
        // over there pressing anything — the claim arrived down a link that
        // device has verified, which is the whole of why it is worth recording.
        self.announce(
            &already,
            &DeviceIdentity {
                device: held.device.clone(),
                fingerprint: held.fingerprint.clone(),
                name: held.name.clone(),
                os: held.os.clone(),
                addresses: held.addresses.clone(),
            },
        )
        .await;

        // And where this device is in the middle of a changeover of its own, the
        // newcomer is one more member that has not acknowledged the certificate
        // coming in — it has just met the one going out and written *that* down,
        // which is the only one it could have met. Told here rather than left for
        // the next start, because until it holds both it is a member the
        // changeover is waiting on and the whole cluster would be waiting with it.
        // Nothing at all where no changeover is in flight, which is every join.
        self.announce_renewal().await;

        Ok(())
    }

    /// Tell each of `members` about `newcomer`, and write down the tellings
    /// that did not get through.
    ///
    /// **Nothing here can fail the press.** The human pressed Allow and the
    /// newcomer is a member of this device whatever some third machine made of
    /// it, so a member that answers nothing is dimmed, is written down as still
    /// owed the telling, and the next one is dialled. A join that failed
    /// because somebody's laptop was shut would be a link nobody could make
    /// while a member was away.
    ///
    /// **And nothing here retries.** The debt is the record, and what pays it
    /// is the next thing that finds that member answering — which is
    /// [`Devices::caught_up`], run against each member this did get through to.
    async fn announce(&self, members: &[verkstead_store::Member], newcomer: &DeviceIdentity) {
        for member in members {
            match self.peers.announce(member, newcomer).await {
                Ok(()) => {
                    tracing::info!(
                        device = %member.device,
                        newcomer = %newcomer.device,
                        "a member has been told about the device that just joined",
                    );

                    // Whatever it was owed about this device is paid, which is
                    // the ordinary case owing nothing and this clearing
                    // nothing.
                    if let Err(why) = self.members.told(&member.device, &newcomer.device).await {
                        tracing::error!(
                            %why,
                            "an announcement that was made could not be cleared as made",
                        );
                    }

                    // And this member is answering, which is the whole of what
                    // a debt was waiting on.
                    self.caught_up(member).await;
                }

                Err(why) => {
                    tracing::info!(
                        %why,
                        device = %member.device,
                        newcomer = %newcomer.device,
                        "a member could not be told about the device that just joined, so it \
                         is owed the telling until it answers again",
                    );

                    if let Err(why) = self
                        .members
                        .owed(&member.device, &newcomer.device, Telling::Joined)
                        .await
                    {
                        tracing::error!(
                            %why,
                            "an announcement that was not made could not be written down as owed",
                        );
                    }
                }
            }
        }
    }

    /// Tell every member that has yet to acknowledge it about the certificate
    /// this device is changing over to, and finish the changeover if that was the
    /// last of them (ADR-0020, *The certificate is renewed before it runs out*).
    ///
    /// **Run at a start, because a start is when a certificate is made again.**
    /// The re-issue happens as the identity is read — see [`Device::renewed`] —
    /// and this is the other half of it: without the telling, the day this device
    /// began presenting the new certificate would be the day every link it holds
    /// stopped working, which is the failure the whole changeover exists to
    /// prevent. Nothing at all where no changeover is in flight, which is all but
    /// one start in a certificate's life.
    ///
    /// **A task of its own rather than something the start waits on.** A member
    /// that is switched off costs a dial's patience apiece down its addresses, and
    /// a Verkstead that would not finish coming up until somebody's laptop had
    /// been answered for would be a changeover costing exactly what it is for.
    ///
    /// **Made presenting the outgoing certificate**, which is not a choice: it is
    /// the only one of the two any member holds, so it is the only one that gets
    /// through a member's gate — see [`Peers::announce_renewal`].
    ///
    /// **And a member that could not be told is owed the telling**, exactly as one
    /// that could not be told about a newcomer or an unlink is, on the same record
    /// and in the same words — see [`Devices::caught_up`], which is what pays it
    /// the moment that member is found answering. Nothing here retries and nothing
    /// here fails: a machine that never answers leaves the changeover in flight,
    /// the old certificate going out, and the startup line naming both
    /// fingerprints, which is a device the human unlinks.
    pub async fn announce_renewal(&self) {
        let Some(incoming) = self.device.incoming_fingerprint() else {
            return;
        };

        let owed = match self.members.yet_to_acknowledge(incoming).await {
            Ok(owed) => owed,

            Err(why) => {
                tracing::error!(%why, "which members are owed a new fingerprint could not be read");

                return;
            }
        };

        if owed.is_empty() {
            // Which is a changeover the start that read the identity already
            // finished, or one this start found nobody owed: either way there is
            // nothing to tell and nothing to settle.
            return;
        }

        tracing::info!(
            fingerprint = %incoming,
            owed = owed.len(),
            "telling the members that have yet to hold this device's new certificate",
        );

        let saying = self.renewing(incoming).await;

        for member in &owed {
            match self.peers.announce_renewal(member, &saying).await {
                Ok(()) => self.acknowledged(member, incoming).await,

                Err(why) => {
                    tracing::info!(
                        %why,
                        device = %member.device,
                        "a member could not be told this device's certificate has been made \
                         again, so it is owed the telling until it answers again",
                    );

                    if let Err(why) = self
                        .members
                        .owed(&member.device, self.device.id(), Telling::Renewed)
                        .await
                    {
                        tracing::error!(
                            %why,
                            "a renewal that was not announced could not be written down as owed",
                        );
                    }
                }
            }
        }

        self.changeover_settled().await;
    }

    /// The same announcement, set going in a task of its own rather than waited
    /// on.
    ///
    /// **For the two places a device is written into this membership by somebody
    /// else's call**: a member naming a newcomer — see
    /// [`crate::peer::announcing`] — and the roster an Exchange hands over — see
    /// [`crate::peer::exchange`]. A device recorded there has acknowledged
    /// nothing, so it is one more member a changeover in flight is waiting on;
    /// and nothing over there would ever tell it, because the announcement is
    /// this device's own to make.
    ///
    /// Left undone it is not a link that breaks but a changeover that cannot
    /// finish: the member holding it up is perfectly reachable, so the old
    /// certificate goes on going out until the next start reads the file again —
    /// and the old certificate is the one with at most [`RENEW_WITHIN`] left on
    /// it. [`Devices::allow`] has always done this at the end of its own press,
    /// for exactly this reason; these two are the same moment arriving from the
    /// other direction.
    ///
    /// **A task, because the caller is a route a peer is waiting on.** A member
    /// that is switched off costs a dial's patience apiece down its addresses,
    /// and an announcement that would not be answered until somebody's laptop had
    /// been dialled for would be a call the announcing device gave up on.
    ///
    /// Nothing at all where no changeover is in flight, which is every recording
    /// on every device but the one in the middle of one.
    pub(crate) fn announcing_renewal(&self) {
        if self.device.incoming_fingerprint().is_none() {
            return;
        }

        let devices = self.clone();

        tokio::spawn(async move { devices.announce_renewal().await });
    }

    /// What an announcement of the renewal carries: this device as it answers
    /// anybody, and the fingerprint of the certificate it is changing to.
    ///
    /// The identity keeps the meaning it has everywhere else — the fingerprint in
    /// it is the certificate this call is *made* under, which the receiver checks
    /// against its own handshake exactly as it would on any other exchange — and
    /// the incoming one is beside it. See
    /// [`verkstead_render::RenewedCertificate`].
    async fn renewing(&self, incoming: &str) -> RenewedCertificate {
        RenewedCertificate {
            identity: self.reading.identity(&self.device).await,
            incoming: incoming.to_owned(),
        }
    }

    /// Write down that `member` holds the certificate coming in, and clear the
    /// debt that said it did not.
    ///
    /// Both, because they are one fact told to the two things that ask after it:
    /// the changeover counts what a member has acknowledged, and a member coming
    /// back after a week reads what it is owed. And a member that is answering has
    /// earned the rest of what it missed, which is the third thing here.
    async fn acknowledged(&self, member: &verkstead_store::Member, incoming: &str) {
        tracing::info!(
            device = %member.device,
            fingerprint = %incoming,
            "a member holds this device's new certificate",
        );

        if let Err(why) = self.members.acknowledged(&member.device, incoming).await {
            tracing::error!(%why, "an acknowledgement that was given could not be written down");
        }

        if let Err(why) = self.members.told(&member.device, self.device.id()).await {
            tracing::error!(%why, "an announcement that was made could not be cleared as made");
        }

        self.caught_up(member).await;
    }

    /// And finish the changeover where the last member has acknowledged: the
    /// certificate that was waiting becomes the one this Data Directory holds, and
    /// the file it was waiting in goes.
    ///
    /// **By the same path a re-issue with nobody to tell takes at the start that
    /// began it** — see [`Device::changed_over`]. One changeover ends one way.
    ///
    /// **What it does not do is switch what this process presents.** The listener
    /// stands behind the certificate it was built with and every dial is made with
    /// a clone of the handle that was read at the start, so the outgoing one goes
    /// on going out until the next start reads the file this just wrote. Which
    /// costs nothing, because every member has acknowledged the new one and kept
    /// the old beside it for exactly this — a member that had let go of it the
    /// moment it acknowledged would be a member refusing the device it had just
    /// acknowledged, for as long as the last machine in the cluster stayed
    /// switched off.
    async fn changeover_settled(&self) {
        let Some(incoming) = self.device.incoming_fingerprint() else {
            return;
        };

        match self.members.unacknowledged(incoming).await {
            Ok(0) => match self.device.changeover_complete() {
                Ok(()) => tracing::info!(
                    fingerprint = %incoming,
                    "every member holds this device's new certificate, so the changeover is \
                     over and the next start presents it",
                ),

                Err(why) => tracing::error!(
                    %why,
                    "the changeover could not be finished, so the next start finishes it",
                ),
            },

            Ok(owed) => tracing::info!(
                fingerprint = %incoming,
                owed,
                "this device's certificate is still changing over, so the old one goes on \
                 going out",
            ),

            Err(why) => {
                tracing::error!(%why, "how many members are owed a new fingerprint could not be read")
            }
        }
    }

    /// **Unlink**: take `device` out of the cluster, for everybody.
    ///
    /// **A membership rather than a set of pairs** (ADR-0020), so the press is
    /// not this device cutting its own half of a link: every member drops the
    /// same device, and the device itself is told to let go of the lot of them.
    /// Cutting one pair was rejected for what it leaves behind — a list that
    /// reads differently depending on which machine you open it on.
    ///
    /// **Three things happen, and the order is the one thing about them that is
    /// forced.** The leaver is told first, while this device still holds a
    /// membership for it to be dialled from and it still holds one for this
    /// device to get through its gate with; then it is dropped here, which is
    /// the human's own machine being right whatever else fails; then every
    /// other member is told. A leaver told after it had been dropped would be a
    /// dial with no row to make it from, and one told after this device had
    /// been dropped over *there* would be a caller that member refuses.
    ///
    /// **Nothing waits on the leaver answering**, which is most of what Unlink
    /// is for: the machine the human reaches for this on is the one that is
    /// never coming back. It is told where it can be, and the cluster is right
    /// either way.
    ///
    /// **And a member that could not be told is owed the removal**, exactly as
    /// one that could not be told about a newcomer is — see
    /// [`Devices::caught_up`], which is what pays either debt the moment that
    /// member is found answering.
    ///
    /// Put `call` to `device` over the link this one holds to it, and hand back
    /// what that device answered (ADR-0020, *The opened device relays*).
    ///
    /// **Two Device Ids are not dialled**, and both are refused by name rather
    /// than tried. One that is no member's is a device this machine knows
    /// nothing about — there is no certificate to pin a dial on and no address
    /// to make it at, and saying so is a different thing from a path that is
    /// wrong. And this device's own id is refused too: local URLs keep their
    /// shape, so nothing should ever ask, and a relay that quietly answered its
    /// own id would be a second way of spelling every local call.
    ///
    /// **Everything else is the dial this tree already makes to a member** —
    /// this device's certificate presented, the member's fingerprint pinned,
    /// and every address it advertised tried in the order it advertised them,
    /// so a device that has moved is reached at its later address. A dial that
    /// answered nowhere leaves the row unreachable the way any other does. See
    /// [`Peers::relay`].
    pub(crate) async fn relay(
        &self,
        device: &str,
        call: crate::relaying::Call,
    ) -> std::result::Result<reqwest::Response, Unrelayed> {
        if device == self.device.id() {
            return Err(Unrelayed::ThisDevice);
        }

        let held = self.members.rows().await.map_err(Unrelayed::Unreadable)?;

        let Some(member) = held.into_iter().find(|member| member.device == device) else {
            return Err(Unrelayed::NoSuchMember);
        };

        self.peers
            .relay(&member, &call)
            .await
            .map_err(Unrelayed::Unreachable)
    }

    /// **A second press is not a second thing happening.** A device that is not
    /// a member is one this machine has already unlinked, and saying so twice
    /// is not a failure — the stance [`verkstead_store::forget_member`] takes,
    /// for its reason.
    pub(crate) async fn unlink(&self, device: &str) -> Result<()> {
        let held = self.members.rows().await?;

        let Some(leaver) = held.iter().find(|member| member.device == device).cloned() else {
            return Ok(());
        };

        // Told while it is still a member here and this device is still one
        // there. A laptop that is not there is told nothing, and nothing waits
        // on it — the cluster it was in has moved on without it, and its own
        // list is wrong until somebody presses Add again.
        if let Err(why) = self.peers.unlink(&leaver, device).await {
            tracing::info!(
                %why,
                device = %device,
                "a device that was unlinked could not be told, so it goes on holding a \
                 cluster that no longer holds it",
            );
        }

        self.members.forget(device).await?;

        tracing::info!(
            device = %device,
            name = %leaver.name,
            "a device has been taken out of this one's cluster",
        );

        for member in held.iter().filter(|member| member.device != device) {
            match self.peers.unlink(member, device).await {
                Ok(()) => {
                    tracing::info!(
                        device = %member.device,
                        leaver = %device,
                        "a member has been told to drop the device that was unlinked",
                    );

                    self.caught_up(member).await;
                }

                Err(why) => {
                    tracing::info!(
                        %why,
                        device = %member.device,
                        leaver = %device,
                        "a member could not be told to drop the device that was unlinked, so \
                         it is owed the telling until it answers again",
                    );

                    if let Err(why) = self
                        .members
                        .owed(&member.device, device, Telling::Removed)
                        .await
                    {
                        tracing::error!(
                            %why,
                            "a removal that was not made could not be written down as owed",
                        );
                    }
                }
            }
        }

        // And the device that has just gone may have been the one a changeover was
        // waiting on — which is the case the changeover has no other answer to: a
        // machine that never comes back is announced to for ever otherwise, and
        // what the task file says about it is that the human unlinks it. So this is
        // where that press finishes the changeover it was holding up.
        self.changeover_settled().await;

        Ok(())
    }

    /// Say to `member` everything it has yet to be told, which is what a dial
    /// that just got through to it has earned the right to do.
    ///
    /// **The trigger is a dial that answered, rather than a timer.** Nothing in
    /// a cluster retries in a loop: a debt is written down when a telling could
    /// not be made, and it is paid the moment that member turns out to be
    /// there. Something dials a member whenever the cluster does anything at
    /// all, so a device whose members are all quiet is a device with nothing
    /// owed that matters yet.
    ///
    /// **Whichever of the three it is**, because a member that was away for a
    /// week may have missed a join, an unlink and a renewal all — see
    /// [`verkstead_store::Telling`], and the reason those are one record rather
    /// than three. A join is said again out of this device's own membership, which
    /// is where the newcomer's addresses and certificate are; a removal carries
    /// nothing but the id; and a renewal is this device's own changeover, which is
    /// read off the handle rather than off a row — a debt naming a changeover this
    /// device is no longer in the middle of is nothing left to say.
    ///
    /// **And a renewal paid here is what finishes a changeover for a member that
    /// was switched off.** The acknowledgement is the answer to the call, so the
    /// member that has just come back may have been the last one owed — which is
    /// the moment the certificate that has been waiting becomes this device's, and
    /// nothing but a dial getting through was ever going to say so.
    ///
    /// **And a member that stops answering part way through keeps the rest.**
    /// The walk stops at the first telling that did not get through, so what is
    /// left is still owed and is said the next time — rather than every
    /// remaining one being dialled at a machine that has just gone.
    ///
    /// Nothing here can fail its caller. The press that ran it already
    /// happened, and a debt that could not be paid is a debt.
    async fn caught_up(&self, member: &verkstead_store::Member) {
        let owed = match self.members.owing(&member.device).await {
            Ok(owed) => owed,
            Err(why) => {
                tracing::error!(%why, "what a member is owed could not be read");

                return;
            }
        };

        if owed.is_empty() {
            return;
        }

        // Read once for the whole walk: what a *joined* telling carries is the
        // device as this one holds it, and the membership does not move while
        // the debts are being paid.
        let held = match self.members.rows().await {
            Ok(held) => held,
            Err(why) => {
                tracing::error!(%why, "the membership a debt is paid out of could not be read");

                return;
            }
        };

        // Whether one of the debts paid below was a renewal, which is the one of
        // the three that leaves something to settle afterwards: the member that
        // has just come back may have been the last one the changeover was
        // waiting on.
        let mut renewed = false;

        for (about, telling) in owed {
            let said = match telling {
                Telling::Removed => self.peers.unlink(member, &about).await,

                // This device's own changeover, read off the handle rather than
                // out of a row — the certificate coming in is a file in the Data
                // Directory and not a thing any membership holds. A debt naming a
                // changeover that is over is nothing left to say, and the telling
                // below takes it away.
                Telling::Renewed => match self.device.incoming_fingerprint() {
                    None => Ok(()),

                    Some(incoming) => {
                        let saying = self.renewing(incoming).await;
                        let said = self.peers.announce_renewal(member, &saying).await;

                        if said.is_ok() {
                            renewed = true;

                            if let Err(why) =
                                self.members.acknowledged(&member.device, incoming).await
                            {
                                tracing::error!(
                                    %why,
                                    "an acknowledgement that was given could not be written down",
                                );
                            }
                        }

                        said
                    }
                },

                Telling::Joined => {
                    let Some(newcomer) = held.iter().find(|held| held.device == about) else {
                        // Owed a join about a device this one no longer holds,
                        // which is nothing left to say: the unlink that dropped
                        // it took its debts with it, so this is a row written
                        // between the two reads above rather than anything to
                        // put right.
                        continue;
                    };

                    self.peers.announce(member, &presenting(newcomer)).await
                }
            };

            if let Err(why) = said {
                tracing::info!(
                    %why,
                    device = %member.device,
                    about = %about,
                    "a member stopped answering part way through what it was owed, so the \
                     rest of it waits for the next call that gets through",
                );

                return;
            }

            tracing::info!(
                device = %member.device,
                about = %about,
                "a member that was away has been told what it missed",
            );

            if let Err(why) = self.members.told(&member.device, &about).await {
                tracing::error!(%why, "a telling that was made could not be cleared as made");
            }
        }

        // And where one of them was a renewal, the changeover may have just had
        // its last acknowledgement — which is the whole of what it was waiting on,
        // and what a member coming back after a week finishes.
        if renewed {
            self.changeover_settled().await;
        }
    }

    /// **Deny**: settle the request, record nothing, and say so.
    ///
    /// The same shrug at a second press, and for the same reason: a request
    /// that is not held is one somebody has already answered or one whose ten
    /// minutes ran out, and neither is a thing to fail.
    ///
    /// Nothing is remembered about the device that was refused. A cluster is a
    /// membership rather than a list of verdicts, and a device turned away is
    /// free to ask again — which is what somebody who pressed the wrong button
    /// would have it do.
    ///
    /// **But it is told.** The ADR spelled the dial back out on an Allow and
    /// left this one, and without it the device that asked reads *waiting*
    /// until somebody over there gets bored and cancels; the human settled that
    /// a refusal comes back the same way. What it carries is that there is
    /// nothing coming and nothing else — a refusal is not a fact about this
    /// cluster to be handed to a stranger.
    pub(crate) async fn deny(&self, request: &str) -> Result<()> {
        let Some(held) = self.joins.held(request, OffsetDateTime::now_utc()).await? else {
            return Ok(());
        };

        self.joins.let_go(request).await?;

        tracing::info!(
            device = %held.device,
            name = %held.name,
            request = %request,
            "a device asking to be let into this one's cluster was refused",
        );

        if let Err(why) = self.peers.settle(&held, &JoinSettled::Denied).await {
            tracing::info!(
                %why,
                device = %held.device,
                request = %request,
                "a device that was refused could not be told, and its own row runs out \
                 inside the ten minutes",
            );
        }

        Ok(())
    }
}

/// A member as it is described to another device: what it said about itself, with
/// **the certificate it is presenting** rather than the one this device has
/// recorded against it.
///
/// **Those are two different strings for exactly as long as that member is in the
/// middle of a changeover of its own.** A device that has re-issued its
/// certificate presents the outgoing one until the last of its own members has
/// acknowledged the new one, so the fingerprint a row is keyed on can be a
/// certificate nothing on that machine is offering yet — see
/// [`verkstead_store::Member::renewing_from`], which is the one it *is* offering.
///
/// And a roster or an announcement is read by a device that has met neither: what
/// it does with this is complete a handshake against it. Handed the one that is
/// not being presented, a newcomer would hold a fingerprint that refused every
/// call the device it names made, until that device restarted — where handed the
/// one going out it can talk at once, and the renewal is announced to it like any
/// other member's, because it has acknowledged nothing.
///
/// One function rather than the mapping written at each place that hands a member
/// over, because it is one judgement and the places are two: the roster an Allow
/// hands to a newcomer, and the announcement made to each member about it.
fn presenting(member: &verkstead_store::Member) -> DeviceIdentity {
    DeviceIdentity {
        device: member.device.clone(),
        fingerprint: member
            .renewing_from
            .clone()
            .unwrap_or_else(|| member.fingerprint.clone()),
        name: member.name.clone(),
        os: member.os.clone(),
        addresses: member.addresses.clone(),
    }
}

/// What `path` holds, where it holds anything: the file's text trimmed, and
/// nothing for a file that is not there or is empty.
///
/// An empty file counts as one that is not there — see [`Device::issued`].
/// Nothing writes one, the writes here being atomic, so it is a machine that
/// lost power or a hand that emptied it, and inventing is the only recovery
/// either of those has.
///
/// **A file that *is* there and will not open is a failure**, though, and that
/// is the one place this parts company with the Workbench Key's own reading.
/// The key takes any bytes at all and a lost one is re-issued by a press; an
/// identity is what a cluster has pinned, and writing a fresh one over a
/// certificate that was sitting right there is the single act in this module
/// that cannot be taken back. A file this process is not allowed to open — one
/// left owned by root by a single start under `sudo`, most of all — is not a
/// file that is missing, and telling the two apart is the whole of the
/// difference between a start that stops and an install that quietly becomes a
/// different device. So the start says which file it was and stops, the way one
/// that will not parse does.
fn read_back(path: &Path) -> std::io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(None),
        Ok(text) => Ok(Some(text.trim().to_owned())),

        Err(gone) if gone.kind() == std::io::ErrorKind::NotFound => Ok(None),

        Err(why) => Err(std::io::Error::other(format!(
            "reading {}: {why} — a file that is there and cannot be read is not a file that \
             is missing, so this start stops rather than inventing an identity over it. Put \
             it back within reach of the account this server runs as, or delete it and let \
             the next start make a fresh one — which every device this one is linked to \
             would then have to be linked to again",
            path.display(),
        ))),
    }
}

/// Sixteen bytes of the operating system's own randomness as lower-case hex.
///
/// What a Device Id is, and what a pending join is named by too — see
/// [`crate::peer::joining`]. One shape for both because they go the same places:
/// into a URL segment, into a log line, and in front of a person reading one off
/// a screen.
///
/// Hex rather than the base64 the Workbench Key is spelled in, which is the one
/// place the two part company. The id goes into a URL segment *and* into the
/// certificate's own subject and subject alternative name, and `_` — which the
/// URL-safe base64 alphabet has — is not a character a host name may contain.
/// Hex has nothing either of those has to escape, and it is the alphabet a
/// person reading an id off a screen is least likely to mistype.
pub(crate) fn invented() -> std::io::Result<String> {
    let mut bytes = [0u8; ID_BYTES];

    getrandom::fill(&mut bytes).map_err(std::io::Error::other)?;

    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Take `path` away, counting one that is already gone as one taken away.
///
/// Which is what a changeover completing twice looks like: a start that
/// finished one and lost power before the file went, and the start after it
/// doing the same work again. The file is in the same state either way, and
/// the second start has nothing to report about the first.
fn taken_away(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        otherwise => otherwise,
    }
}

/// A fresh self-signed certificate for `id`, good for `good_for`, and the key
/// that signed it, as the one PEM text a file holds.
///
/// **The id is what the certificate is named after**, in the subject's common
/// name and as its one subject alternative name. It is the only name this
/// device has that does not change: the hostname is what a device is *shown*
/// under and two machines may share one, where the id was invented to be a
/// device's own — which is why a re-issue is a new certificate for the same
/// device rather than a new device.
///
/// **It carries both purposes.** One certificate stands at both ends of every
/// link — a device presents it to answer a call and presents the same one to
/// make a call — so a certificate good for only one of the two would be a
/// device that could be talked to and could not talk.
///
/// The validity is counted from the moment it is made, and every certificate a
/// start makes is made for [`VALIDITY`]: `good_for` is said rather than taken
/// from there so that a fixture can stand a suite at a start near the expiry —
/// see [`Device::stated_good_for`]. Nothing is backdated: a clock skewed far
/// enough to refuse a certificate issued now is a clock that will refuse the
/// handshake's other checks too, and an hour quietly added here would make the
/// ninety days ninety days and an hour.
fn minted(id: &str, good_for: time::Duration) -> std::io::Result<String> {
    let key = KeyPair::generate().map_err(std::io::Error::other)?;

    let mut params = CertificateParams::new(vec![id.to_owned()]).map_err(std::io::Error::other)?;

    params.distinguished_name = DistinguishedName::new();
    params.distinguished_name.push(DnType::CommonName, id);

    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ServerAuth,
        ExtendedKeyUsagePurpose::ClientAuth,
    ];

    let now = OffsetDateTime::now_utc();

    params.not_before = now;
    params.not_after = now + good_for;

    let certificate = params.self_signed(&key).map_err(std::io::Error::other)?;

    Ok(format!("{}{}", key.serialize_pem(), certificate.pem()))
}

/// The certificate in `pem` as this module holds one, and the parse that has to
/// happen to hold it.
///
/// There is no reading either of the two things beside the text without a
/// parse: the fingerprint is a hash of the certificate's DER — its own bytes,
/// as they go over the wire, rather than of the file's text, so that what two
/// people compare is what every other tool would print of the same certificate
/// and does not move when a line ending does — and the expiry is a field
/// inside it, which is where a re-issue reads whether it is due.
///
/// The private key is parsed and thrown away. Nothing here wants it; what
/// asking for it buys is that a truncated file is a failure at the start that
/// read it rather than a handshake that will not complete weeks later.
fn held(pem: &str) -> anyhow::Result<Held> {
    // Ended with one newline whichever way it arrived. What is read back off
    // the disk is trimmed — an empty file is a file that is not there, and that
    // judgement is made on the text — so without this the certificate a restart
    // holds would differ from the one the start that wrote it held by the line
    // ending at the end of it.
    let certificate = format!("{}\n", pem.trim_end());

    PrivateKeyDer::from_pem_slice(certificate.as_bytes())
        .context("the private key that should stand in front of it")?;

    let der =
        CertificateDer::from_pem_slice(certificate.as_bytes()).context("the certificate itself")?;

    let (_, parsed) = X509Certificate::from_der(&der)
        .map_err(|why| anyhow!("{why}"))
        .context("what the certificate says about itself")?;

    let expires = OffsetDateTime::from_unix_timestamp(parsed.validity().not_after.timestamp())
        .context("the expiry the certificate carries")?;

    Ok(Held {
        fingerprint: fingerprint_of_der(&der),
        certificate,
        expires,
    })
}

/// The same fingerprint of a certificate that arrived rather than one that was
/// read off a file: the bytes are already the DER, there being no PEM around
/// them on the wire.
///
/// Said here rather than at the other end, because the two have to be the one
/// spelling. A member list is keyed by what [`Device::fingerprint`] prints and
/// a caller's certificate is checked against it — see [`crate::peer::Caller`] —
/// so a second spelling would be a comparison between two true strings about
/// the same certificate that never matched.
pub(crate) fn fingerprint_of_der(certificate: &CertificateDer<'_>) -> String {
    printable(Sha256::digest(certificate))
}

/// A digest in the spelling a person compares by eye: upper-case hex, in pairs,
/// colon-separated.
fn printable(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<String>>()
        .join(":")
}
