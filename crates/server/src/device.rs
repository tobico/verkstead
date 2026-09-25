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
//! identified. Here it is made, read back and named; the peer listener that
//! presents it and the renewal that replaces it come after.
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
//! **The validity is said here rather than taken from the crate.** Ninety days,
//! written down in [`VALIDITY`]. An expired certificate is refused at the
//! handshake, so the number decides when every link in a cluster would go down
//! together if nothing renewed it — which is exactly why a default accepted
//! without looking was the thing ADR-0020 ruled out, along with a validity long
//! enough never to matter.

use std::path::{Path, PathBuf};

use rcgen::{CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, KeyPair};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

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

/// And what both are written as: readable and writable by the account Verkstead
/// runs under, and by nothing else on the machine. The same mode the Workbench
/// Key's file gets, and for the same reason on the half that is a private key —
/// it is the whole of what proves this device is this device, and a link is
/// somebody else's machine trusting it.
///
/// The id is nobody's secret and is written at that mode anyway. It is one
/// identity in two files, and two modes over it would be two things to
/// remember and one of them to get wrong.
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
/// Nothing here renews: the re-issue that reads this is the renewal's own, and
/// what this stage does with the number is write it into the certificate.
pub const VALIDITY: time::Duration = time::Duration::days(90);

/// This device: its id, its certificate, and the two files they are kept in.
///
/// A handle rather than the two strings, because the files are what the
/// identity *is*: a second Verkstead over the same Data Directory is the same
/// device, and a re-issue has somewhere to write.
#[derive(Debug, Clone)]
pub struct Device {
    /// The directory both files are in, which is the Data Directory. Kept
    /// rather than the two paths, because either is the other's neighbour and
    /// the renewal writes one of them again.
    dir: PathBuf,

    id: String,

    /// The private key and the certificate, as the file holds them: the text
    /// is what is presented and what is read back, so it is kept rather than
    /// reassembled out of parsed halves.
    certificate: String,

    /// And the certificate's fingerprint, worked out once as it is read or
    /// made. It is compared by eye rather than by machine — two people on two
    /// phones checking that the device one of them is linking is the device the
    /// other is offering — so it is a string here rather than the digest it is
    /// a spelling of.
    fingerprint: String,
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
    /// A certificate file that is there and will *not* parse is a failure
    /// rather than a fresh certificate. That is the one place this parts
    /// company with the key, which takes any bytes at all: a certificate is
    /// either read or is nothing, and quietly issuing a new one over the one a
    /// cluster has pinned is the single act here that cannot be taken back. So
    /// the start says what it could not read and what deleting the file would
    /// do, and stops.
    pub fn issued(data_dir: &Path) -> std::io::Result<Device> {
        let id_path = data_dir.join(ID_FILE);
        let certificate_path = data_dir.join(CERTIFICATE_FILE);

        // The id first, and whether it was already there — because the
        // certificate follows from it. A certificate is made out to an id, so
        // one found beside an id that had to be invented is a certificate for a
        // device that no longer exists, and a fresh id takes a fresh
        // certificate with it.
        let (id, kept) = match read_back(&id_path) {
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
            true => read_back(&certificate_path),
            false => None,
        };

        let certificate = match certificate {
            Some(certificate) => certificate,
            None => {
                let certificate = minted(&id)?;

                write_atomically(&certificate_path, &certificate, DEVICE_MODE)?;

                certificate
            }
        };

        Device::holding(data_dir, id, certificate)
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
        write_atomically(&data_dir.join(ID_FILE), &format!("{id}\n"), DEVICE_MODE)?;

        let certificate = minted(id)?;

        write_atomically(&data_dir.join(CERTIFICATE_FILE), &certificate, DEVICE_MODE)?;

        Device::holding(data_dir, id.to_owned(), certificate)
    }

    /// One handle over an id and a certificate that are both already on disk.
    ///
    /// Where the certificate is parsed, which is what turns text into an
    /// identity: the fingerprint is a hash of the certificate's own bytes
    /// rather than of the file's, so there is no reading this without parsing,
    /// and the private key is parsed beside it because half a pair would be
    /// found out at the first handshake instead of here.
    fn holding(data_dir: &Path, id: String, certificate: String) -> std::io::Result<Device> {
        // Ended with one newline whichever way it arrived. What is read back
        // off the disk is trimmed — an empty file is a file that is not there,
        // and that judgement is made on the text — so without this the
        // certificate a restart holds would differ from the one the start that
        // wrote it held by the line ending at the end of it.
        let certificate = format!("{}\n", certificate.trim_end());

        let fingerprint = fingerprint_of(&certificate).map_err(|why| {
            std::io::Error::other(format!(
                "reading the device certificate in {}: {why} — delete {CERTIFICATE_FILE} and \
                 the next start makes a fresh one, which every device this one is linked to \
                 would then have to be linked to again",
                data_dir.display(),
            ))
        })?;

        Ok(Device {
            dir: data_dir.to_owned(),
            id,
            certificate,
            fingerprint,
        })
    }

    /// What every record and URL names this device by.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The certificate and its private key, as the file holds them: the PEM a
    /// TLS configuration is built out of.
    pub fn certificate(&self) -> &str {
        &self.certificate
    }

    /// The certificate's fingerprint, in the spelling a human compares one in:
    /// the SHA-256 of its bytes as upper-case hex in colon-separated pairs,
    /// which is what every tool that has ever printed one prints.
    ///
    /// That spelling is the point of it. The fingerprint is what one person
    /// reads off a phone while another reads it off a screen, so it is grouped
    /// to be found a place in and cased to be read aloud.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Where the id is kept, which is what says so in a failure to write it.
    pub fn id_path(&self) -> PathBuf {
        self.dir.join(ID_FILE)
    }

    /// And where the certificate is.
    pub fn certificate_path(&self) -> PathBuf {
        self.dir.join(CERTIFICATE_FILE)
    }
}

/// What `path` holds, where it holds anything: the file's text trimmed, and
/// nothing for a file that is not there or is empty.
///
/// An empty file counts as one that is not there — see [`Device::issued`] — and
/// so does one that could not be read at all, which is the same judgement the
/// Workbench Key makes: a directory this server cannot read is a directory it
/// cannot write either, so the write that follows is what reports it, naming
/// the file it was trying to make rather than the one it failed to find.
fn read_back(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(text) if !text.trim().is_empty() => Some(text.trim().to_owned()),
        _ => None,
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

/// A fresh self-signed certificate for `id`, and the key that signed it, as the
/// one PEM text the file holds.
///
/// **The id is what the certificate is named after**, in the subject's common
/// name and as its one subject alternative name. It is the only name this
/// device has that does not change: the hostname is what a device is *shown*
/// under and two machines may share one, where the id was invented to be a
/// device's own.
///
/// **It carries both purposes.** One certificate stands at both ends of every
/// link — a device presents it to answer a call and presents the same one to
/// make a call — so a certificate good for only one of the two would be a
/// device that could be talked to and could not talk.
///
/// The validity is [`VALIDITY`], counted from the moment it is made. Nothing is
/// backdated: a clock skewed far enough to refuse a certificate issued now is a
/// clock that will refuse the handshake's other checks too, and an hour quietly
/// added here would make the ninety days ninety days and an hour.
fn minted(id: &str) -> std::io::Result<String> {
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
    params.not_after = now + VALIDITY;

    let certificate = params.self_signed(&key).map_err(std::io::Error::other)?;

    Ok(format!("{}{}", key.serialize_pem(), certificate.pem()))
}

/// The fingerprint of the certificate in `pem`, and the parse that has to
/// happen to work one out.
///
/// The hash is of the certificate's DER — its own bytes, as they go over the
/// wire — rather than of the file's text, so that the fingerprint two people
/// compare is the one every other tool would print of the same certificate and
/// does not move when a line ending does.
///
/// The private key is parsed and thrown away. Nothing here wants it yet; what
/// asking for it buys is that a truncated file is a failure at the start that
/// wrote it rather than a handshake that will not complete weeks later.
fn fingerprint_of(pem: &str) -> Result<String, rustls_pki_types::pem::Error> {
    PrivateKeyDer::from_pem_slice(pem.as_bytes())?;

    let certificate = CertificateDer::from_pem_slice(pem.as_bytes())?;

    Ok(fingerprint_of_der(&certificate))
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
