//! What the store keeps about a join in flight (ADR-0020, *The join*).
//!
//! **Two tables, and the point of the suite is that they are two.** What the
//! device that pressed Add keeps and what the device it asked keeps are not the
//! same record: one knows the address it typed and the certificate it met, the
//! other knows the whole of what a stranger said about itself. So each is
//! written and read back for what it is, and neither is asked about the other's
//! columns.
//!
//! **And that the ten minutes are a moment rather than a length.** A request is
//! written down with when it runs out, and a process that came up afterwards
//! reads the same moment: the whole reason the request is a row is that a
//! restart halfway through somebody confirming it is not a request silently
//! dropped, and a clock counted from the start would be that dropping made
//! quieter.

use sqlx::SqlitePool;
use verkstead_store::{
    AskedJoin, HeldJoin, ask_join, asked_join, asked_joins, forget_asked_join, held_join,
    hold_join, let_go_of_expired_joins, let_go_of_join, open_database,
};

/// The device asking, named by the id a cluster names one by.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And a second, for the question about two requests at once.
const C: &str = "ffeeddccbbaa00998877665544332211";

/// The certificate each presents, spelled the way this tree spells a
/// fingerprint: what is pinned into a request is this string.
const A_FINGERPRINT: &str = "3A:7B:1F:04:C8:92:6D:5E:AA:11:B0:47:9C:3D:2E:88";
const C_FINGERPRINT: &str = "5E:6D:92:C8:04:1F:7B:3A:88:2E:3D:9C:47:B0:11:AA";

/// And what the far end calls the requests, which is what a cancel names.
const A_REQUEST: &str = "1122334455667788";
const ANOTHER_REQUEST: &str = "8877665544332211";

/// A moment well behind any test run, and one well ahead of it: the two sides
/// of *has this run out*.
const LONG_AGO: &str = "2020-01-01T00:00:00Z";
const LONG_HENCE: &str = "2099-01-01T00:00:00Z";

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A join as the device that asked writes one down.
fn asking(request: &str, address: &str) -> AskedJoin {
    AskedJoin {
        request: request.to_owned(),
        address: address.to_owned(),
        device: C.to_owned(),
        name: "workbench".to_owned(),
        fingerprint: C_FINGERPRINT.to_owned(),
        asked_at: "2026-09-25T10:00:00Z".to_owned(),
        expires_at: LONG_HENCE.to_owned(),
    }
}

/// And as the device it was asked of holds one: the whole of what the stranger
/// said, and the certificate the handshake took from it.
fn held(request: &str, expires_at: &str) -> HeldJoin {
    HeldJoin {
        request: request.to_owned(),
        device: A.to_owned(),
        name: "laptop".to_owned(),
        os: "macOS".to_owned(),
        addresses: vec![
            "laptop.tailnet-name.ts.net".to_owned(),
            "100.64.0.2".to_owned(),
            "192.168.1.31".to_owned(),
        ],
        fingerprint: A_FINGERPRINT.to_owned(),
        asked_at: "2026-09-25T10:00:00Z".to_owned(),
        expires_at: expires_at.to_owned(),
    }
}

/// What the device that asked keeps: where it knocked, what answered, and when
/// the question runs out.
#[tokio::test]
async fn a_join_this_device_asked_for_reads_back_whole() {
    let (_dir, pool) = fresh_pool().await;

    assert!(asked_joins(&pool).await.unwrap().is_empty());

    let asked = asking(A_REQUEST, "192.168.1.31");
    ask_join(&pool, &asked).await.unwrap();

    assert_eq!(asked_joins(&pool).await.unwrap(), vec![asked.clone()]);
    assert_eq!(asked_join(&pool, A_REQUEST).await.unwrap(), Some(asked));
    assert_eq!(asked_join(&pool, ANOTHER_REQUEST).await.unwrap(), None);
}

/// And what the device that was asked keeps: the whole of what the stranger
/// said about itself, with its addresses in the order it advertised them.
///
/// The order is load-bearing for the same reason a member's is — the dial back
/// that answers an Allow works down that list, and the tailnet half is the half
/// that crosses.
#[tokio::test]
async fn a_join_this_device_is_holding_reads_back_whole() {
    let (_dir, pool) = fresh_pool().await;

    let holding = held(A_REQUEST, LONG_HENCE);
    hold_join(&pool, &holding).await.unwrap();

    let read = held_join(&pool, A_REQUEST).await.unwrap().unwrap();

    assert_eq!(read, holding);
    assert_eq!(
        read.addresses,
        vec!["laptop.tailnet-name.ts.net", "100.64.0.2", "192.168.1.31"],
        "in the order they were advertised, which is the order a dial back works \
         down them in",
    );
    assert_eq!(read.fingerprint, A_FINGERPRINT);
}

/// A request that has run out still reads back with the moment it ran out at.
///
/// **The store does not judge it**, which is the whole of why this is a test: an
/// expired request is refused rather than missing, and the two are told apart by
/// reading the moment rather than by a read coming back empty. A store that hid
/// one would leave the route unable to say which it was looking at.
#[tokio::test]
async fn an_expired_request_is_still_a_row_with_its_moment_on_it() {
    let (_dir, pool) = fresh_pool().await;

    hold_join(&pool, &held(A_REQUEST, LONG_AGO)).await.unwrap();

    let read = held_join(&pool, A_REQUEST).await.unwrap().unwrap();

    assert_eq!(
        read.expires_at, LONG_AGO,
        "the moment is the far end's to read, and it is the moment the request \
         was made plus ten minutes rather than anything a restart worked out",
    );
}

/// And the sweep takes those and leaves the ones still being held.
#[tokio::test]
async fn the_sweep_takes_the_ones_that_ran_out() {
    let (_dir, pool) = fresh_pool().await;

    hold_join(&pool, &held(A_REQUEST, LONG_AGO)).await.unwrap();
    hold_join(&pool, &held(ANOTHER_REQUEST, LONG_HENCE))
        .await
        .unwrap();

    let_go_of_expired_joins(&pool, "2026-09-25T10:05:00Z")
        .await
        .unwrap();

    assert_eq!(held_join(&pool, A_REQUEST).await.unwrap(), None);
    assert!(
        held_join(&pool, ANOTHER_REQUEST).await.unwrap().is_some(),
        "a request still inside its ten minutes is a question still being held",
    );
}

/// Letting go of one takes its addresses with it, so nothing is left pointing
/// at a request that is not there.
#[tokio::test]
async fn letting_go_takes_the_addresses_with_it() {
    let (_dir, pool) = fresh_pool().await;

    hold_join(&pool, &held(A_REQUEST, LONG_HENCE))
        .await
        .unwrap();
    let_go_of_join(&pool, A_REQUEST).await.unwrap();

    assert_eq!(held_join(&pool, A_REQUEST).await.unwrap(), None);

    // And the same request may be held again afterwards, which it could not be
    // if an address were still naming it.
    hold_join(&pool, &held(A_REQUEST, LONG_HENCE))
        .await
        .unwrap();

    assert!(held_join(&pool, A_REQUEST).await.unwrap().is_some());
}

/// Letting go of a request that is not there is not a thing to fail, on either
/// side of the asking.
///
/// Which is what makes a second Cancel nothing new: the row went the first time,
/// and saying so twice is one thing said twice.
#[tokio::test]
async fn letting_go_twice_is_not_a_failure() {
    let (_dir, pool) = fresh_pool().await;

    ask_join(&pool, &asking(A_REQUEST, "192.168.1.31"))
        .await
        .unwrap();

    forget_asked_join(&pool, A_REQUEST).await.unwrap();
    forget_asked_join(&pool, A_REQUEST).await.unwrap();

    assert!(asked_joins(&pool).await.unwrap().is_empty());

    let_go_of_join(&pool, ANOTHER_REQUEST).await.unwrap();
}

/// And the two tables are two: what one side is holding says nothing about what
/// the other side asked, even under the one request id.
#[tokio::test]
async fn the_two_sides_are_two_records() {
    let (_dir, pool) = fresh_pool().await;

    ask_join(&pool, &asking(A_REQUEST, "192.168.1.31"))
        .await
        .unwrap();

    assert_eq!(
        held_join(&pool, A_REQUEST).await.unwrap(),
        None,
        "asking for a join is not being asked for one",
    );

    hold_join(&pool, &held(ANOTHER_REQUEST, LONG_HENCE))
        .await
        .unwrap();
    let_go_of_join(&pool, ANOTHER_REQUEST).await.unwrap();

    assert!(
        asked_join(&pool, A_REQUEST).await.unwrap().is_some(),
        "and letting go of one this device was holding leaves the one it asked \
         for exactly where it was",
    );
}
