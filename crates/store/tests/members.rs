//! What the store keeps about the devices this one is linked to (ADR-0020, *A
//! cluster is a membership*).
//!
//! The row every later stage of linking writes into, asked the three questions
//! the membership is asked of it: whether a caller's certificate is a member's,
//! how many there are, and what the Devices section draws. And the one thing
//! about it that is not a column — the addresses are a *list*, and a peer dials
//! them in the order the far end advertised them, so an order that came back
//! any other way would be a dial reaching for the LAN before the tailnet.

use sqlx::SqlitePool;
use verkstead_store::{
    Linking, forget_member, member_count, member_holding, members, open_database, record_member,
};

/// The two devices these tests link to, named by the ids a cluster names them
/// by rather than by anything read off a machine.
const B: &str = "0011223344556677889900aabbccddee";
const C: &str = "ffeeddccbbaa00998877665544332211";

/// And the certificates they present, spelled the way this tree spells a
/// fingerprint: what a member *is* on the peer listener is this string, so a
/// test about a membership compares one.
const B_FINGERPRINT: &str = "3A:7B:1F:04:C8:92:6D:5E:AA:11:B0:47:9C:3D:2E:88";
const C_FINGERPRINT: &str = "5E:6D:92:C8:04:1F:7B:3A:88:2E:3D:9C:47:B0:11:AA";

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A device saying what it is, as one does on every exchange: the tailnet half
/// of its addresses first, then the LAN.
fn advertising(device: &str, name: &str, fingerprint: &str) -> Linking {
    Linking {
        device: device.to_owned(),
        name: name.to_owned(),
        os: "Linux".to_owned(),
        addresses: vec![
            format!("{name}.tailnet-name.ts.net"),
            "100.64.0.2".to_owned(),
            "192.168.1.31".to_owned(),
        ],
        fingerprint: fingerprint.to_owned(),
    }
}

/// A device written down is a device the membership holds, said by everything
/// that asks after one.
#[tokio::test]
async fn a_recorded_device_is_a_member() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(member_count(&pool).await.unwrap(), 0);
    assert!(!member_holding(&pool, B_FINGERPRINT).await.unwrap());

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    assert_eq!(member_count(&pool).await.unwrap(), 1);
    assert!(
        member_holding(&pool, B_FINGERPRINT).await.unwrap(),
        "a caller presenting this certificate is one of ours",
    );

    let listed = members(&pool).await.unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].device, B);
    assert_eq!(listed[0].name, "workbench");
    assert_eq!(listed[0].os, "Linux");
    assert_eq!(listed[0].fingerprint, B_FINGERPRINT);
    assert!(
        !listed[0].last_seen.is_empty(),
        "a member is recorded off an exchange that just got through, so the \
         moment is written by the recording",
    );
}

/// And a certificate nothing here has recorded is nobody's, which is what the
/// member gate refuses on.
#[tokio::test]
async fn an_unrecorded_certificate_is_nobodys() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    assert!(
        !member_holding(&pool, C_FINGERPRINT).await.unwrap(),
        "one member's certificate is not another's — a membership is a set of \
         fingerprints and nothing else",
    );
}

/// The addresses come back out in the order they went in.
///
/// Which is the whole of why they are a table with a position rather than a
/// column: a device advertises all of them on every exchange and a peer works
/// down the list, so an order shuffled by the database would be a dial reaching
/// for the LAN before the tailnet — and the tailnet is the half that crosses.
#[tokio::test]
async fn the_addresses_come_back_in_the_order_they_went_in() {
    let (_dir, pool) = fresh_pool().await;

    let advertised = vec![
        "workbench.tailnet-name.ts.net".to_owned(),
        "100.64.0.2".to_owned(),
        "fd7a:115c:a1e0::2".to_owned(),
        "192.168.1.31".to_owned(),
        "10.0.0.7".to_owned(),
    ];

    record_member(
        &pool,
        &Linking {
            addresses: advertised.clone(),
            ..advertising(B, "workbench", B_FINGERPRINT)
        },
    )
    .await
    .unwrap();

    assert_eq!(members(&pool).await.unwrap()[0].addresses, advertised);
}

/// A device recorded again is the same member said again rather than a second
/// row, and what it now says replaces what it said before.
///
/// Keyed by the id because that is what outlives both halves of the rest: a
/// certificate is renewed and an address moves, and the id lasts as long as the
/// far end's Data Directory.
#[tokio::test]
async fn a_device_recorded_again_is_the_same_member() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    record_member(
        &pool,
        &Linking {
            name: "workbench-renamed".to_owned(),
            addresses: vec!["10.0.0.9".to_owned()],
            fingerprint: C_FINGERPRINT.to_owned(),
            ..advertising(B, "workbench", B_FINGERPRINT)
        },
    )
    .await
    .unwrap();

    let listed = members(&pool).await.unwrap();

    assert_eq!(listed.len(), 1, "one device is one member");
    assert_eq!(listed[0].name, "workbench-renamed");
    assert_eq!(listed[0].fingerprint, C_FINGERPRINT);
    assert_eq!(
        listed[0].addresses,
        vec!["10.0.0.9"],
        "the addresses are what the far end just advertised rather than those \
         and whatever it used to say",
    );

    assert!(
        !member_holding(&pool, B_FINGERPRINT).await.unwrap(),
        "and the certificate it has stopped presenting is nobody's again",
    );
}

/// A member taken out of the table is out of it: not held, not counted and not
/// drawn.
#[tokio::test]
async fn a_forgotten_member_is_gone() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();
    record_member(&pool, &advertising(C, "laptop", C_FINGERPRINT))
        .await
        .unwrap();

    forget_member(&pool, B).await.unwrap();

    assert!(!member_holding(&pool, B_FINGERPRINT).await.unwrap());
    assert_eq!(member_count(&pool).await.unwrap(), 1);

    let listed = members(&pool).await.unwrap();

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].device, C);
    assert_eq!(
        listed[0].addresses.len(),
        3,
        "and the addresses that went with the one that left took none of the \
         remaining member's with them",
    );

    // And a device that is not a member is already not a member: unlinking one
    // twice is not a thing to fail.
    forget_member(&pool, B).await.unwrap();
}
