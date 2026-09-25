//! What this Verkstead says it is: the device id and the self-signed
//! certificate made at the first start and read back at every one after
//! (ADR-0020).
//!
//! A suite standing where a start does, and asking the two questions a restart
//! asks — is this the same device, and is the certificate the same one — along
//! with what the certificate turned out to carry. The certificate is read back
//! with `x509-parser` rather than with the crate that wrote it: what is being
//! checked is that the validity and the device id reached the certificate, and
//! reading those out of rcgen's own parameters would check nothing.
//!
//! The Workbench Key's own suite is `tests/workbench_key.rs`, which is the same
//! shape of file about the same shape of problem — a credential in a file of its
//! own under the Data Directory.

use std::path::Path;

use rustls_pki_types::CertificateDer;
use rustls_pki_types::pem::PemObject;
use sha2::{Digest, Sha256};
use verkstead_server::device::{Device, VALIDITY};
use verkstead_server::key::WorkbenchKey;
use x509_parser::prelude::FromDer;

/// The two files the identity is, named here so that a test can empty one or
/// take it away. The module names them for itself and says why; a suite about
/// what happens to them has to be able to say which.
const ID_FILE: &str = "device.id";
const CERTIFICATE_FILE: &str = "device.pem";

/// A Data Directory with nothing in it, which is what a first start sees.
fn fresh() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// The certificate this device holds, as the bytes that go over the wire.
fn der(device: &Device) -> CertificateDer<'static> {
    CertificateDer::from_pem_slice(device.certificate().as_bytes())
        .expect("the certificate this device was made with should parse")
}

#[test]
fn a_second_start_reads_the_first_starts_id_and_certificate() {
    let dir = fresh();

    let first = Device::issued(dir.path()).unwrap();
    let second = Device::issued(dir.path()).unwrap();

    assert_eq!(
        first.id(),
        second.id(),
        "an id invented at every start would be a device no record could name twice",
    );
    assert_eq!(
        first.fingerprint(),
        second.fingerprint(),
        "and a certificate made at every start would be a link that a reboot took down",
    );
    assert_eq!(
        first.certificate(),
        second.certificate(),
        "the second start should have read the file rather than written one",
    );
}

#[test]
fn deleting_the_files_makes_fresh_ones() {
    let dir = fresh();

    let first = Device::issued(dir.path()).unwrap();

    std::fs::remove_file(dir.path().join(ID_FILE)).unwrap();
    std::fs::remove_file(dir.path().join(CERTIFICATE_FILE)).unwrap();

    let second = Device::issued(dir.path()).unwrap();

    assert_ne!(
        first.id(),
        second.id(),
        "with nothing on disk there is nothing to read back, and inventing is the recovery",
    );
    assert_ne!(first.fingerprint(), second.fingerprint());
}

#[test]
fn an_empty_file_counts_as_one_that_is_not_there() {
    let dir = fresh();

    let first = Device::issued(dir.path()).unwrap();

    // Nothing writes one — both writes are atomic — so an empty file is a
    // machine that lost power or a hand that emptied it.
    std::fs::write(dir.path().join(ID_FILE), "").unwrap();
    std::fs::write(dir.path().join(CERTIFICATE_FILE), "").unwrap();

    let second = Device::issued(dir.path()).unwrap();

    assert_ne!(first.id(), second.id());
    assert_ne!(first.fingerprint(), second.fingerprint());

    // And whitespace is the same thing: a file holding a newline holds no id.
    std::fs::write(dir.path().join(ID_FILE), "\n").unwrap();

    let third = Device::issued(dir.path()).unwrap();

    assert_ne!(second.id(), third.id());
}

#[test]
fn an_id_with_no_certificate_beside_it_keeps_the_id() {
    let dir = fresh();

    let first = Device::issued(dir.path()).unwrap();

    // Which is what a start that wrote the id and then failed to write the
    // certificate leaves behind.
    std::fs::remove_file(dir.path().join(CERTIFICATE_FILE)).unwrap();

    let second = Device::issued(dir.path()).unwrap();

    assert_eq!(
        first.id(),
        second.id(),
        "the id is the half every record names, and it was there to be read",
    );
    assert_ne!(
        first.fingerprint(),
        second.fingerprint(),
        "and the certificate is made out to it",
    );
}

#[test]
fn a_certificate_with_no_id_beside_it_goes_with_the_id() {
    let dir = fresh();

    let first = Device::issued(dir.path()).unwrap();

    std::fs::remove_file(dir.path().join(ID_FILE)).unwrap();

    let second = Device::issued(dir.path()).unwrap();

    assert_ne!(first.id(), second.id());
    assert_ne!(
        first.fingerprint(),
        second.fingerprint(),
        "a certificate made out to an id that is gone is a certificate for nobody",
    );
    assert_eq!(
        common_name(&second),
        second.id(),
        "and the one that replaced it names the device that is there",
    );
}

#[test]
fn a_certificate_that_will_not_parse_is_a_failure_rather_than_a_fresh_one() {
    let dir = fresh();

    Device::issued(dir.path()).unwrap();

    std::fs::write(dir.path().join(CERTIFICATE_FILE), "not a certificate\n").unwrap();

    let failed = Device::issued(dir.path()).expect_err(
        "issuing a new certificate over the one a cluster has pinned is the one act here \
         that cannot be taken back, so a file that will not parse stops the start",
    );

    let said = failed.to_string();

    assert!(
        said.contains(CERTIFICATE_FILE),
        "the failure should name the file to delete, and said: {said}",
    );
}

#[test]
#[cfg(unix)]
fn both_files_are_written_at_the_workbench_keys_own_mode() {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh();

    let device = Device::issued(dir.path()).unwrap();
    let key = WorkbenchKey::issued(dir.path()).unwrap();

    let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;

    let keys = mode(key.path());

    assert_eq!(keys, 0o600, "which is what the key's own module says it is");

    for path in [device.id_path(), device.certificate_path()] {
        assert_eq!(
            mode(&path),
            keys,
            "{} is half of what proves this device is this device",
            path.display(),
        );
    }
}

#[test]
fn a_stated_identity_is_the_id_the_suite_gave_it() {
    let dir = fresh();

    let stated = Device::stated(dir.path(), "a-stated-device").unwrap();

    assert_eq!(stated.id(), "a-stated-device");
    assert_eq!(
        common_name(&stated),
        "a-stated-device",
        "the certificate is made out to the id whatever wrote the id",
    );

    // And it is on disk the way an invented one is, so a start after the
    // fixture reads what the fixture said.
    let read_back = Device::issued(dir.path()).unwrap();

    assert_eq!(read_back.id(), "a-stated-device");
    assert_eq!(read_back.fingerprint(), stated.fingerprint());
}

#[test]
fn the_certificate_is_good_for_ninety_days() {
    let dir = fresh();

    let device = Device::issued(dir.path()).unwrap();
    let der = der(&device);
    let (_, certificate) = x509_parser::certificate::X509Certificate::from_der(&der).unwrap();

    let validity = certificate.validity();
    let good_for = validity.not_after.timestamp() - validity.not_before.timestamp();

    assert_eq!(
        good_for,
        VALIDITY.whole_seconds(),
        "the validity is Verkstead's own decision rather than whatever the crate defaults to",
    );
    assert_eq!(
        VALIDITY,
        time::Duration::days(90),
        "and it is ninety days, which is the number a renewal is scheduled against",
    );
}

#[test]
fn the_certificate_is_named_after_the_device_id() {
    let dir = fresh();

    let device = Device::issued(dir.path()).unwrap();

    assert_eq!(
        common_name(&device),
        device.id(),
        "the id is the one name this device has that does not change",
    );

    let der = der(&device);
    let (_, certificate) = x509_parser::certificate::X509Certificate::from_der(&der).unwrap();

    let names: Vec<String> = certificate
        .subject_alternative_name()
        .unwrap()
        .expect("a certificate that named nothing could not be dialled")
        .value
        .general_names
        .iter()
        .map(|name| name.to_string())
        .collect();

    assert!(
        names.iter().any(|name| name.contains(device.id())),
        "the id should be the certificate's own name too, and it said: {names:?}",
    );
}

#[test]
fn the_certificate_is_good_at_both_ends_of_a_link() {
    let dir = fresh();

    let device = Device::issued(dir.path()).unwrap();
    let der = der(&device);
    let (_, certificate) = x509_parser::certificate::X509Certificate::from_der(&der).unwrap();

    let usage = certificate
        .extended_key_usage()
        .unwrap()
        .expect("one certificate stands at both ends of every link")
        .value;

    assert!(usage.server_auth, "a device answers a call with it");
    assert!(usage.client_auth, "and makes one with it");
}

#[test]
fn the_fingerprint_is_the_certificates_own_digest_spelled_for_the_eye() {
    let dir = fresh();

    let device = Device::issued(dir.path()).unwrap();

    let digest = Sha256::digest(der(&device));
    let expected = digest
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<String>>()
        .join(":");

    assert_eq!(
        device.fingerprint(),
        expected,
        "the hash is of the certificate's own bytes, so every other tool prints the same one",
    );

    // Which is the spelling two people compare in: thirty-two pairs, grouped
    // to be found a place in and cased to be read aloud.
    assert_eq!(device.fingerprint().split(':').count(), 32);
    assert!(
        device
            .fingerprint()
            .chars()
            .all(|character| character == ':' || character.is_ascii_hexdigit()),
    );
    assert_eq!(device.fingerprint(), device.fingerprint().to_uppercase());
}

#[test]
fn the_id_is_short_enough_to_sit_in_a_url_segment() {
    let dir = fresh();

    let device = Device::issued(dir.path()).unwrap();

    // Which is what every record and URL in a cluster names a device by, so it
    // carries nothing a URL or a host name would have to escape — the hostname
    // and the tailnet name were rejected as ids, and an id that needed quoting
    // would be a third thing to regret.
    assert_eq!(device.id().len(), 32);
    assert!(
        device
            .id()
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit()),
        "the id said {}",
        device.id(),
    );
}

/// The common name the certificate `device` holds was made out to.
fn common_name(device: &Device) -> String {
    let der = der(device);
    let (_, certificate) = x509_parser::certificate::X509Certificate::from_der(&der).unwrap();

    certificate
        .subject()
        .iter_common_name()
        .next()
        .expect("a certificate is made out to the device's id")
        .as_str()
        .unwrap()
        .to_owned()
}
