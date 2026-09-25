//! What this Verkstead says it is: the device id and the self-signed
//! certificate made at the first start and read back at every one after
//! (ADR-0020).
//!
//! A suite standing where a start does, and asking the questions a restart
//! asks — is this the same device, is the certificate the same one, and is it
//! near enough its expiry to be made again — along with what the certificate
//! turned out to carry. The certificate is read back with `x509-parser` rather
//! than with the crate that wrote it: what is being checked is that the
//! validity and the device id reached the certificate, and reading those out of
//! rcgen's own parameters would check nothing.
//!
//! **The renewal is stood at by stating how much life a certificate was made
//! with**, rather than by moving a clock this process does not keep — see
//! [`Device::stated_good_for`]. A certificate made with twenty-nine days in it
//! is a certificate twenty-nine days from expiry, and what a start does with
//! one is what it would do on a machine that had been running for two months.
//!
//! The Workbench Key's own suite is `tests/workbench_key.rs`, which is the same
//! shape of file about the same shape of problem — a credential in a file of its
//! own under the Data Directory.

use std::path::Path;

use rustls_pki_types::CertificateDer;
use rustls_pki_types::pem::PemObject;
use sha2::{Digest, Sha256};
use verkstead_server::device::{Changeover, Device, RENEW_WITHIN, VALIDITY};
use verkstead_server::key::WorkbenchKey;
use verkstead_server::peer::Members;
use x509_parser::prelude::FromDer;

/// The two files the identity is, named here so that a test can empty one or
/// take it away. The module names them for itself and says why; a suite about
/// what happens to them has to be able to say which.
const ID_FILE: &str = "device.id";
const CERTIFICATE_FILE: &str = "device.pem";

/// And the file a certificate waits in over a changeover, which the suite about
/// the renewal has to be able to empty and to look for.
const INCOMING_FILE: &str = "device.next.pem";

/// The id the renewal's own tests state their device as, so that what they
/// assert against is a string they chose — see [`Device::stated`], which is
/// here for that reason.
const A_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// A day either side of the renewal window, which is what stands a start just
/// inside it or just outside. Said against `RENEW_WITHIN` rather than as a
/// number of days, so that a suite about the window moves with it.
const A_DAY: time::Duration = time::Duration::days(1);

/// A Data Directory with nothing in it, which is what a first start sees.
fn fresh() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// The certificate this device holds, as the bytes that go over the wire.
fn der(device: &Device) -> CertificateDer<'static> {
    CertificateDer::from_pem_slice(device.certificate().as_bytes())
        .expect("the certificate this device was made with should parse")
}

#[tokio::test]
async fn a_second_start_reads_the_first_starts_id_and_certificate() {
    let dir = fresh();

    let first = Device::issued(dir.path(), &Members::none()).await.unwrap();
    let second = Device::issued(dir.path(), &Members::none()).await.unwrap();

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

#[tokio::test]
async fn deleting_the_files_makes_fresh_ones() {
    let dir = fresh();

    let first = Device::issued(dir.path(), &Members::none()).await.unwrap();

    std::fs::remove_file(dir.path().join(ID_FILE)).unwrap();
    std::fs::remove_file(dir.path().join(CERTIFICATE_FILE)).unwrap();

    let second = Device::issued(dir.path(), &Members::none()).await.unwrap();

    assert_ne!(
        first.id(),
        second.id(),
        "with nothing on disk there is nothing to read back, and inventing is the recovery",
    );
    assert_ne!(first.fingerprint(), second.fingerprint());
}

#[tokio::test]
async fn an_empty_file_counts_as_one_that_is_not_there() {
    let dir = fresh();

    let first = Device::issued(dir.path(), &Members::none()).await.unwrap();

    // Nothing writes one — both writes are atomic — so an empty file is a
    // machine that lost power or a hand that emptied it.
    std::fs::write(dir.path().join(ID_FILE), "").unwrap();
    std::fs::write(dir.path().join(CERTIFICATE_FILE), "").unwrap();

    let second = Device::issued(dir.path(), &Members::none()).await.unwrap();

    assert_ne!(first.id(), second.id());
    assert_ne!(first.fingerprint(), second.fingerprint());

    // And whitespace is the same thing: a file holding a newline holds no id.
    std::fs::write(dir.path().join(ID_FILE), "\n").unwrap();

    let third = Device::issued(dir.path(), &Members::none()).await.unwrap();

    assert_ne!(second.id(), third.id());
}

#[tokio::test]
async fn an_id_with_no_certificate_beside_it_keeps_the_id() {
    let dir = fresh();

    let first = Device::issued(dir.path(), &Members::none()).await.unwrap();

    // Which is what a start that wrote the id and then failed to write the
    // certificate leaves behind.
    std::fs::remove_file(dir.path().join(CERTIFICATE_FILE)).unwrap();

    let second = Device::issued(dir.path(), &Members::none()).await.unwrap();

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

#[tokio::test]
async fn a_certificate_with_no_id_beside_it_goes_with_the_id() {
    let dir = fresh();

    let first = Device::issued(dir.path(), &Members::none()).await.unwrap();

    std::fs::remove_file(dir.path().join(ID_FILE)).unwrap();

    let second = Device::issued(dir.path(), &Members::none()).await.unwrap();

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

#[tokio::test]
async fn a_certificate_that_will_not_parse_is_a_failure_rather_than_a_fresh_one() {
    let dir = fresh();

    Device::issued(dir.path(), &Members::none()).await.unwrap();

    std::fs::write(dir.path().join(CERTIFICATE_FILE), "not a certificate\n").unwrap();

    let failed = Device::issued(dir.path(), &Members::none())
        .await
        .expect_err(
            "issuing a new certificate over the one a cluster has pinned is the one act here \
         that cannot be taken back, so a file that will not parse stops the start",
        );

    let said = failed.to_string();

    assert!(
        said.contains(CERTIFICATE_FILE),
        "the failure should name the file to delete, and said: {said}",
    );
}

/// And a file that is there and will not *open* is a failure too, which is the
/// half a missing file and an unreadable one are told apart by.
///
/// The case this is really about is a `device.pem` left owned by root by one
/// start under `sudo`: the file is right there and this process cannot read it,
/// while the directory around it stays writable — so treating it as a file that
/// is not there is not a write that fails and reports itself, it is a rename
/// straight over the certificate every linked device is holding.
///
/// Stood at with a directory in the file's place rather than with a mode,
/// because a suite that ran as root would read a mode of nothing perfectly
/// well and prove the opposite of what it says. What is being asked is that an
/// error which is not *not found* stops the start, and any of them does.
#[tokio::test]
async fn a_file_that_will_not_open_is_a_failure_rather_than_a_fresh_one() {
    for file in [ID_FILE, CERTIFICATE_FILE] {
        let dir = fresh();

        let first = Device::issued(dir.path(), &Members::none()).await.unwrap();

        std::fs::remove_file(dir.path().join(file)).unwrap();
        std::fs::create_dir(dir.path().join(file)).unwrap();

        let failed = Device::issued(dir.path(), &Members::none())
            .await
            .expect_err(
                "a file that is there and cannot be read is not a file that is missing, and \
             writing a fresh identity over one is the act that cannot be taken back",
            );

        let said = failed.to_string();

        assert!(
            said.contains(file),
            "the failure should name the file it could not read, and said: {said}",
        );

        // And nothing was written over on the way out: the id is still the one
        // this device was invented as.
        if file != ID_FILE {
            assert_eq!(
                std::fs::read_to_string(dir.path().join(ID_FILE))
                    .unwrap()
                    .trim(),
                first.id(),
            );
        }
    }
}

/// And the same of the certificate waiting out a changeover, which is read by
/// the same reading.
#[tokio::test]
async fn a_changeover_file_that_will_not_open_is_a_failure_too() {
    let dir = fresh();

    Device::stated_good_for(dir.path(), A_DEVICE, RENEW_WITHIN - A_DAY).unwrap();

    let started = Device::issued(dir.path(), &Members::stated(1))
        .await
        .unwrap();

    std::fs::remove_file(started.incoming_path()).unwrap();
    std::fs::create_dir(started.incoming_path()).unwrap();

    let failed = Device::issued(dir.path(), &Members::stated(1))
        .await
        .expect_err("a changeover cannot be carried on out of a file nothing can read");

    assert!(
        failed.to_string().contains(INCOMING_FILE),
        "the failure should name the file it could not read, and said: {failed}",
    );
}

#[tokio::test]
#[cfg(unix)]
async fn both_files_are_written_at_the_workbench_keys_own_mode() {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh();

    let device = Device::issued(dir.path(), &Members::none()).await.unwrap();
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

#[tokio::test]
async fn a_stated_identity_is_the_id_the_suite_gave_it() {
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
    let read_back = Device::issued(dir.path(), &Members::none()).await.unwrap();

    assert_eq!(read_back.id(), "a-stated-device");
    assert_eq!(read_back.fingerprint(), stated.fingerprint());
}

#[tokio::test]
async fn the_certificate_is_good_for_ninety_days() {
    let dir = fresh();

    let device = Device::issued(dir.path(), &Members::none()).await.unwrap();
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

#[tokio::test]
async fn the_certificate_is_named_after_the_device_id() {
    let dir = fresh();

    let device = Device::issued(dir.path(), &Members::none()).await.unwrap();

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

#[tokio::test]
async fn the_certificate_is_good_at_both_ends_of_a_link() {
    let dir = fresh();

    let device = Device::issued(dir.path(), &Members::none()).await.unwrap();
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

#[tokio::test]
async fn the_fingerprint_is_the_certificates_own_digest_spelled_for_the_eye() {
    let dir = fresh();

    let device = Device::issued(dir.path(), &Members::none()).await.unwrap();

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

#[tokio::test]
async fn the_id_is_short_enough_to_sit_in_a_url_segment() {
    let dir = fresh();

    let device = Device::issued(dir.path(), &Members::none()).await.unwrap();

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

#[test]
fn the_renewal_window_is_thirty_of_the_ninety_days() {
    assert_eq!(
        RENEW_WITHIN,
        time::Duration::days(30),
        "which leaves two months of ordinary starts to make the certificate again in, \
         and a month of them after in which to notice that one did",
    );
    assert!(
        RENEW_WITHIN < VALIDITY,
        "a window as wide as the validity would be a certificate re-issued at every start",
    );
}

#[tokio::test]
async fn a_start_with_the_expiry_near_makes_the_certificate_again() {
    let dir = fresh();

    let near = Device::stated_good_for(dir.path(), A_DEVICE, RENEW_WITHIN - A_DAY).unwrap();
    let started = Device::issued(dir.path(), &Members::none()).await.unwrap();

    assert_ne!(
        started.fingerprint(),
        near.fingerprint(),
        "an expired certificate is refused at the handshake, so a device that only ever \
         read its own file back would take every link it has down on one day",
    );

    // And the fresh one is what is on disk from here: a start after this reads
    // it back rather than making a third.
    let after = Device::issued(dir.path(), &Members::none()).await.unwrap();

    assert_eq!(after.fingerprint(), started.fingerprint());
    assert_eq!(
        after.changeover(),
        Changeover::NotDue,
        "the certificate it just read has its whole ninety days",
    );
}

#[tokio::test]
async fn a_start_with_the_expiry_far_off_leaves_the_certificate_alone() {
    let dir = fresh();

    let far = Device::stated_good_for(dir.path(), A_DEVICE, RENEW_WITHIN + A_DAY).unwrap();
    let started = Device::issued(dir.path(), &Members::none()).await.unwrap();

    assert_eq!(
        started.fingerprint(),
        far.fingerprint(),
        "a day outside the window is outside it: a re-issue at every start would be a \
         fingerprint every member had to be told about every time this process came up",
    );
    assert_eq!(started.changeover(), Changeover::NotDue);
    assert_eq!(started.certificate(), far.certificate());
}

#[tokio::test]
async fn with_no_members_the_changeover_completes_at_once_and_says_it_had_nobody_to_tell() {
    let dir = fresh();

    let old = Device::stated_good_for(dir.path(), A_DEVICE, RENEW_WITHIN - A_DAY).unwrap();
    let started = Device::issued(dir.path(), &Members::none()).await.unwrap();

    assert_eq!(
        started.changeover(),
        Changeover::NobodyToTell,
        "a member is made by a join and there is no join yet, so there is nobody owed an \
         announcement of the new fingerprint and nothing to wait for",
    );
    assert_ne!(started.fingerprint(), old.fingerprint());
    assert_eq!(
        started.incoming_fingerprint(),
        None,
        "a changeover that completed is over: there is one certificate again",
    );
    assert!(
        !started.incoming_path().exists(),
        "and the file the new one waits in is taken away with it",
    );
}

#[tokio::test]
async fn a_member_yet_to_acknowledge_keeps_the_old_certificate_going_out() {
    let dir = fresh();

    let old = Device::stated_good_for(dir.path(), A_DEVICE, RENEW_WITHIN - A_DAY).unwrap();

    // One member, which has acknowledged nothing — because there is nothing
    // anywhere yet for an acknowledgement to be recorded in, the announcement
    // being the linking stage's.
    let started = Device::issued(dir.path(), &Members::stated(1))
        .await
        .unwrap();

    assert_eq!(
        started.changeover(),
        Changeover::YetToTell(1),
        "the changeover is in flight rather than over",
    );
    assert_eq!(
        started.fingerprint(),
        old.fingerprint(),
        "the changeover never costs a call: what the member holds is what it is answered \
         with, until it has said it holds the other",
    );

    let incoming = started
        .incoming_fingerprint()
        .expect("a fresh certificate was made, it is just not the one going out yet")
        .to_owned();

    assert_ne!(incoming, old.fingerprint());

    // Both printable over the changeover, which is how anybody tells which of
    // the two a peer met.
    assert_ne!(started.fingerprint(), incoming);

    // A restart in the middle holds the same pair. A third certificate here
    // would be a third fingerprint for the members to acknowledge, and a
    // changeover that never finished.
    let again = Device::issued(dir.path(), &Members::stated(1))
        .await
        .unwrap();

    assert_eq!(again.fingerprint(), old.fingerprint());
    assert_eq!(again.incoming_fingerprint(), Some(incoming.as_str()));
    assert_eq!(again.changeover(), Changeover::YetToTell(1));

    // And when nobody is owed one any more, the certificate that was waiting
    // becomes the one presented.
    let done = Device::issued(dir.path(), &Members::none()).await.unwrap();

    assert_eq!(done.fingerprint(), incoming);
    assert_eq!(done.incoming_fingerprint(), None);
    assert_eq!(done.changeover(), Changeover::NobodyToTell);
    assert!(!done.incoming_path().exists());
}

#[tokio::test]
#[cfg(unix)]
async fn the_certificate_waiting_out_a_changeover_is_written_at_the_same_mode() {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh();

    Device::stated_good_for(dir.path(), A_DEVICE, RENEW_WITHIN - A_DAY).unwrap();

    let started = Device::issued(dir.path(), &Members::stated(1))
        .await
        .unwrap();
    let mode = std::fs::metadata(started.incoming_path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(
        mode, 0o600,
        "it is a private key beside a certificate, the same as the file it will replace",
    );
}

#[tokio::test]
async fn the_device_id_is_untouched_by_a_re_issue() {
    let dir = fresh();

    let before = Device::stated_good_for(dir.path(), A_DEVICE, RENEW_WITHIN - A_DAY).unwrap();
    let started = Device::issued(dir.path(), &Members::none()).await.unwrap();

    assert_eq!(
        started.id(),
        before.id(),
        "it is the certificate that is renewed: a device keeps its id for as long as its \
         Data Directory lasts, which is what every record and URL in a cluster names it by",
    );
    assert_eq!(
        common_name(&started),
        A_DEVICE,
        "and the certificate that replaced it is made out to the same device",
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join(ID_FILE))
            .unwrap()
            .trim(),
        A_DEVICE,
        "the file it is read back out of was never written over",
    );
}

#[tokio::test]
async fn a_certificate_waiting_out_a_changeover_that_will_not_parse_says_what_deleting_it_costs() {
    let dir = fresh();

    Device::stated_good_for(dir.path(), A_DEVICE, RENEW_WITHIN - A_DAY).unwrap();

    let started = Device::issued(dir.path(), &Members::stated(1))
        .await
        .unwrap();

    std::fs::write(started.incoming_path(), "not a certificate\n").unwrap();

    let failed = Device::issued(dir.path(), &Members::stated(1))
        .await
        .expect_err("a certificate that will not parse is a failure rather than a fresh one");

    let said = failed.to_string();

    assert!(
        said.contains(INCOMING_FILE),
        "the failure should name the file to delete, and said: {said}",
    );
    assert!(
        said.contains("costs nothing"),
        "and deleting this one costs nothing, nothing having acknowledged it — which is \
         not what deleting the presented certificate costs, and said: {said}",
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
