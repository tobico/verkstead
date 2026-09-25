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
//! began it, and says it had nobody to tell. Telling them is the linking
//! stage's, there being no member to tell yet.
//!
//! **And [`Devices`] is what the human's own browser reads of all this**: this
//! device and how many others are linked to it, which is the Devices section of
//! the Remote access pane. The same answer a stranger reads off the peer
//! listener's identity endpoint, told to the browser instead — that listener
//! presents a certificate nothing but another Verkstead has a reason to trust,
//! so the workbench is where the pane asks.

pub mod reading;

use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use rcgen::{CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, KeyPair};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use verkstead_render::DevicesView;
use x509_parser::certificate::X509Certificate;
use x509_parser::prelude::FromDer;

use crate::peer::Members;
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
/// Read off the handle rather than logged and forgotten, because it is where
/// the linking stage picks the announcement up: a changeover still owed one is
/// a changeover with a member to tell, and telling them is the whole of what
/// that stage adds to what is here.
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
    /// Which is every re-issue this build can make. A member is made by a join
    /// and there is no join yet — see [`Members`] — so the answer to *is
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
    pub fn issued(data_dir: &Path, members: &Members) -> std::io::Result<Device> {
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

        Device::holding(data_dir, id, certificate)?.renewed(members)
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
    /// whole of the bookkeeping. Telling them is the linking stage's, and
    /// there is no member to tell yet — so what this answers is *nobody*, and
    /// [`Changeover::NobodyToTell`] is it saying so.
    fn renewed(self, members: &Members) -> std::io::Result<Device> {
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

        match members.unacknowledged(&incoming.fingerprint) {
            // Nobody is owed an announcement, so the changeover completes at
            // once: the new certificate becomes the presented one and the file
            // it was waiting in goes. In that order, so that a machine losing
            // power between the two leaves a start holding two copies of one
            // certificate rather than none — which is a changeover that
            // completes again and comes out where this one did.
            0 => {
                write_atomically(&self.certificate_path(), &incoming.certificate, DEVICE_MODE)?;

                taken_away(&self.incoming_path())?;

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

/// What the **Devices** section of the Remote access pane reads: this device,
/// and how many others are linked to it (ADR-0020).
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

    /// And who it is linked to, which is what the count on the card comes off.
    /// Nobody, in every Verkstead this build can make: a member is made by a
    /// join, and the join is the next stage's — see [`crate::peer::Members`].
    members: Members,
}

impl Devices {
    /// The list a server answers out of: what it is, the machine it is on, and
    /// its membership.
    ///
    /// The same three the peer listener is built from, because they are the
    /// same three things — what is different is who is asking.
    pub fn of(device: Device, reading: reading::Reading, members: Members) -> Devices {
        Devices {
            device,
            reading,
            members,
        }
    }

    /// The list as the pane draws it, assembled now.
    ///
    /// `async` for the reason the identity endpoint's handler is: the tailnet
    /// half of the addresses is a command run on this machine, and a pane opened
    /// a moment later would get a different and equally true answer.
    pub(crate) async fn listing(&self) -> DevicesView {
        DevicesView {
            this: self.reading.identity(&self.device).await,
            linked: self.members.count(),
        }
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
/// Hex rather than the base64 the Workbench Key is spelled in, which is the one
/// place the two part company. The id goes into a URL segment *and* into the
/// certificate's own subject and subject alternative name, and `_` — which the
/// URL-safe base64 alphabet has — is not a character a host name may contain.
/// Hex has nothing either of those has to escape, and it is the alphabet a
/// person reading an id off a screen is least likely to mistype.
fn invented() -> std::io::Result<String> {
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
