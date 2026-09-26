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
    Linking, Renewal, Telling, announcement_made, announcements_owed, announcements_owed_to,
    changeover_over, forget_every_member, forget_member, member_count, member_holding,
    member_unreachable, members, members_yet_to_acknowledge, open_database, owe_announcement,
    record_member, record_renewal, renewal_acknowledged,
};

/// The two devices these tests link to, named by the ids a cluster names them
/// by rather than by anything read off a machine.
const B: &str = "0011223344556677889900aabbccddee";
const C: &str = "ffeeddccbbaa00998877665544332211";

/// And the device they are told about, which is on neither of their lists:
/// what a debt names is a device rather than a member of this one.
const A: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And a fourth, for the third thing a member can be owed: a renewal names the
/// device doing the telling, which is this machine and so on nobody's list
/// either.
const D: &str = "1122334455667788990011aabbccddff";

/// And the certificates they present, spelled the way this tree spells a
/// fingerprint: what a member *is* on the peer listener is this string, so a
/// test about a membership compares one.
const B_FINGERPRINT: &str = "3A:7B:1F:04:C8:92:6D:5E:AA:11:B0:47:9C:3D:2E:88";
const C_FINGERPRINT: &str = "5E:6D:92:C8:04:1F:7B:3A:88:2E:3D:9C:47:B0:11:AA";

/// And one more certificate, which is the one being changed *to*: a renewal is a
/// device keeping its id and presenting something else, so a test about one needs
/// a third string that is nobody's yet.
const RENEWED: &str = "C8:04:3A:7B:6D:92:1F:5E:47:B0:88:2E:AA:11:9C:3D";

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

/// And the same device saying it has made its certificate again: the one it is
/// still presenting, and the one it is changing to.
fn renewing(device: &str, name: &str, presenting: &str, incoming: &str) -> Renewal {
    let advertising = advertising(device, name, presenting);

    Renewal {
        device: advertising.device,
        name: advertising.name,
        os: advertising.os,
        addresses: advertising.addresses,
        presenting: presenting.to_owned(),
        incoming: incoming.to_owned(),
    }
}

/// Who is owed an announcement of [`RENEWED`], by id — which is what a changeover
/// is waiting on.
async fn owed_the_new_one(pool: &SqlitePool) -> Vec<String> {
    owed_the_new_one_after(pool, RENEWED).await
}

/// And the same of any certificate, for the question about the renewal *after*
/// this one.
async fn owed_the_new_one_after(pool: &SqlitePool, fingerprint: &str) -> Vec<String> {
    members_yet_to_acknowledge(pool, fingerprint)
        .await
        .unwrap()
        .into_iter()
        .map(|member| member.device)
        .collect()
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

/// A member a dial found nothing at is marked, and nothing else about the row
/// moves.
///
/// Which is what *unreachable* is: the row stays, with the addresses the next
/// dial will work down and the certificate it will check, and the moment it was
/// last really heard from. A dial that heard nothing heard nothing.
#[tokio::test]
async fn a_member_that_answers_nothing_is_marked_and_left_alone() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    let before = members(&pool).await.unwrap().remove(0);

    assert!(
        before.reachable,
        "a member is recorded off an exchange that got through, so it starts out \
         answering",
    );

    member_unreachable(&pool, B).await.unwrap();

    let after = members(&pool).await.unwrap().remove(0);

    assert!(!after.reachable);
    assert_eq!(after.addresses, before.addresses);
    assert_eq!(after.fingerprint, before.fingerprint);
    assert_eq!(after.name, before.name);
    assert_eq!(after.last_seen, before.last_seen);

    assert_eq!(
        member_count(&pool).await.unwrap(),
        1,
        "it is still one of this cluster, and the card counts it",
    );
    assert!(
        member_holding(&pool, B_FINGERPRINT).await.unwrap(),
        "and it is still admitted past the gate: a device nobody can reach can \
         still reach this one, which is a laptop coming back on the tailnet",
    );

    // And a device that is not a member is a device no dial has anything to say
    // about, so marking one is not a thing to fail.
    member_unreachable(&pool, C).await.unwrap();
}

/// And the mark comes off the moment it is recorded again, which is the next
/// dial that got through.
#[tokio::test]
async fn a_marked_member_is_answering_again_when_it_is_recorded() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();
    member_unreachable(&pool, B).await.unwrap();

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    assert!(
        members(&pool).await.unwrap()[0].reachable,
        "a recording is an exchange that just got through, which is the whole of \
         what says a member is answering",
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

// ---------------------------------------------------------------------------
// What a member has yet to be told
// ---------------------------------------------------------------------------

/// A telling that did not get through is owed, and one that did takes the debt
/// away.
///
/// **Which is the whole of what the record is for.** An announcement is a dial,
/// and a machine that is off is reached by nothing — but the thing being
/// announced happened anyway, so what the failure leaves behind is a list of
/// who has not heard rather than a link half made.
#[tokio::test]
async fn a_telling_that_did_not_get_through_is_owed_until_it_does() {
    let (_dir, pool) = fresh_pool().await;

    assert!(
        announcements_owed(&pool, A).await.unwrap().is_empty(),
        "nothing has been announced, so nobody is owed anything",
    );

    owe_announcement(&pool, B, A, Telling::Joined)
        .await
        .unwrap();
    owe_announcement(&pool, C, A, Telling::Joined)
        .await
        .unwrap();

    assert_eq!(
        announcements_owed(&pool, A).await.unwrap(),
        vec![B.to_owned(), C.to_owned()],
        "both members have yet to hear about the device that joined",
    );

    announcement_made(&pool, B, A).await.unwrap();

    assert_eq!(
        announcements_owed(&pool, A).await.unwrap(),
        vec![C.to_owned()],
        "and the one that was told is owed nothing",
    );

    // Owing the same telling twice is owing it once: a row says that this
    // member has not heard about that device, and a second announcement that
    // also failed says the same thing again.
    owe_announcement(&pool, C, A, Telling::Joined)
        .await
        .unwrap();

    assert_eq!(
        announcements_owed(&pool, A).await.unwrap(),
        vec![C.to_owned()]
    );

    // And clearing what was never owed is not a thing to fail, which is the
    // ordinary case: an announcement that got through the first time was never
    // written down.
    announcement_made(&pool, B, A).await.unwrap();
}

/// A debt is about a pair, so a member owed one telling is not owed another.
#[tokio::test]
async fn a_debt_names_the_device_it_is_about() {
    let (_dir, pool) = fresh_pool().await;

    owe_announcement(&pool, C, A, Telling::Joined)
        .await
        .unwrap();
    owe_announcement(&pool, C, B, Telling::Joined)
        .await
        .unwrap();

    assert_eq!(
        announcements_owed(&pool, A).await.unwrap(),
        vec![C.to_owned()]
    );
    assert_eq!(
        announcements_owed(&pool, B).await.unwrap(),
        vec![C.to_owned()]
    );

    announcement_made(&pool, C, A).await.unwrap();

    assert!(
        announcements_owed(&pool, A).await.unwrap().is_empty(),
        "the telling that was made is the one that is paid",
    );
    assert_eq!(
        announcements_owed(&pool, B).await.unwrap(),
        vec![C.to_owned()],
        "and the other is still owed",
    );
}

/// A member taken out of the cluster takes its debts with it, in both
/// directions: what it was owed, and what anybody was owed about it.
#[tokio::test]
async fn an_unlinked_member_is_owed_nothing_and_owed_about_by_nobody() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    owe_announcement(&pool, B, A, Telling::Joined)
        .await
        .unwrap();
    owe_announcement(&pool, C, B, Telling::Joined)
        .await
        .unwrap();

    forget_member(&pool, B).await.unwrap();

    assert!(
        announcements_owed(&pool, A).await.unwrap().is_empty(),
        "a device that is not a member is one nothing here has left to tell",
    );
    assert!(
        announcements_owed(&pool, B).await.unwrap().is_empty(),
        "and a debt naming it would be a dial nobody would ever make",
    );
}

/// A debt says *what* is owed, and the three a cluster makes are a device
/// arriving, a device leaving, and a device's certificate changing.
///
/// **Which is what an unlink needed the word for, and what a renewal reuses.** A
/// member that was away is told what it missed when it comes back, and three
/// records to walk instead of one would be three chances to walk only two of
/// them.
#[tokio::test]
async fn a_debt_says_which_of_the_three_tellings_it_is() {
    let (_dir, pool) = fresh_pool().await;

    owe_announcement(&pool, B, A, Telling::Joined)
        .await
        .unwrap();
    owe_announcement(&pool, B, C, Telling::Removed)
        .await
        .unwrap();
    owe_announcement(&pool, B, D, Telling::Renewed)
        .await
        .unwrap();

    assert_eq!(
        announcements_owed_to(&pool, B).await.unwrap(),
        vec![
            (D.to_owned(), Telling::Renewed),
            (A.to_owned(), Telling::Joined),
            (C.to_owned(), Telling::Removed),
        ],
        "read the other way round: not who has not heard, but what this one \
         has yet to be told — in one order, by the device named, so that a \
         member coming back after a busy week is caught up the same way twice",
    );

    assert!(
        announcements_owed_to(&pool, C).await.unwrap().is_empty(),
        "and a member owed nothing is owed nothing",
    );
}

/// And the later telling replaces the earlier, rather than being ignored for a
/// pair already there.
///
/// A member that was off when a device joined and off again when the human
/// unlinked it is owed the removal alone: it never heard the join, and telling
/// it about a device the cluster no longer holds would be a row it would have
/// to be told to take away again on the next call.
#[tokio::test]
async fn the_later_telling_is_the_one_that_is_owed() {
    let (_dir, pool) = fresh_pool().await;

    owe_announcement(&pool, B, A, Telling::Joined)
        .await
        .unwrap();
    owe_announcement(&pool, B, A, Telling::Removed)
        .await
        .unwrap();

    assert_eq!(
        announcements_owed_to(&pool, B).await.unwrap(),
        vec![(A.to_owned(), Telling::Removed)],
        "one debt about that device, and it is the last thing said",
    );

    announcement_made(&pool, B, A).await.unwrap();

    assert!(
        announcements_owed_to(&pool, B).await.unwrap().is_empty(),
        "and paying the pair pays it whichever of the two it turned out to be",
    );
}

/// A device told it has been unlinked lets go of the whole membership, rather
/// than of the device that told it.
///
/// **The leaver's own half of an unlink.** Every other device in the cluster
/// has dropped it, so every one of them would refuse it at the gate — a leaver
/// that kept whichever members the telling happened to name would be a cluster
/// of one that thought it was a cluster of two.
#[tokio::test]
async fn a_device_told_it_has_left_forgets_everybody() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();
    record_member(&pool, &advertising(C, "laptop", C_FINGERPRINT))
        .await
        .unwrap();

    owe_announcement(&pool, B, A, Telling::Joined)
        .await
        .unwrap();

    forget_every_member(&pool).await.unwrap();

    assert!(
        members(&pool).await.unwrap().is_empty(),
        "the list is this device's own row and nothing else",
    );
    assert_eq!(member_count(&pool).await.unwrap(), 0, "and so is the count");
    assert!(
        !member_holding(&pool, B_FINGERPRINT).await.unwrap(),
        "and nobody is admitted at the gate on a certificate this device no \
         longer holds a membership for",
    );
    assert!(
        announcements_owed_to(&pool, B).await.unwrap().is_empty(),
        "a device that is in no cluster owes nobody anything",
    );

    // And being told twice is not a thing to fail: a device that holds no
    // members has already forgotten everybody.
    forget_every_member(&pool).await.unwrap();
}

// ---------------------------------------------------------------------------
// A member's own changeover
// ---------------------------------------------------------------------------

/// A member that has made its certificate again is the same member under a new
/// fingerprint, and *both* of them get through the gate.
///
/// **Which is the whole of what a changeover needs of this end.** The far end goes
/// on presenting the certificate every one of its own members holds until the last
/// of them has acknowledged the new one, and it tells nobody when that was — so a
/// device that recorded the new fingerprint and let go of the old would be refusing
/// the very device that had just announced it.
#[tokio::test]
async fn a_renewed_member_is_the_same_member_under_two_certificates() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    assert!(
        record_renewal(&pool, &renewing(B, "workbench", B_FINGERPRINT, RENEWED))
            .await
            .unwrap(),
        "a renewal about a device this membership holds is a row it changed",
    );

    let listed = members(&pool).await.unwrap();

    assert_eq!(
        listed.len(),
        1,
        "one member rather than two: the id is the key"
    );
    assert_eq!(listed[0].device, B);
    assert_eq!(
        listed[0].fingerprint, RENEWED,
        "the row is keyed on the certificate coming in from here",
    );
    assert_eq!(
        listed[0].renewing_from.as_deref(),
        Some(B_FINGERPRINT),
        "and the one still going out is kept beside it",
    );

    assert!(
        member_holding(&pool, RENEWED).await.unwrap(),
        "a call under the new certificate gets through",
    );
    assert!(
        member_holding(&pool, B_FINGERPRINT).await.unwrap(),
        "and so does one under the old, which is the only one that device is \
         presenting yet",
    );
}

/// And the same renewal announced twice keeps the certificate that is still going
/// out, rather than taking it for a third one.
///
/// **Which the call has to survive**, because it is made again: an answer that went
/// missing is a call worth making a second time, and a member that was switched off
/// is told when it comes back.
#[tokio::test]
async fn the_same_renewal_twice_keeps_the_certificate_still_going_out() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    for _ in 0..2 {
        record_renewal(&pool, &renewing(B, "workbench", B_FINGERPRINT, RENEWED))
            .await
            .unwrap();
    }

    let listed = members(&pool).await.unwrap();

    assert_eq!(listed[0].fingerprint, RENEWED);
    assert_eq!(
        listed[0].renewing_from.as_deref(),
        Some(B_FINGERPRINT),
        "the second call wrote the same two strings, rather than taking the \
         certificate it had just recorded for the one being changed from",
    );
}

/// A renewal carries the addresses with it, as every exchange does.
#[tokio::test]
async fn a_renewal_advertises_the_addresses_with_it() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    let mut moved = renewing(B, "studio", B_FINGERPRINT, RENEWED);
    moved.addresses = vec![
        "studio.tailnet-name.ts.net".to_owned(),
        "192.168.1.40".to_owned(),
    ];

    record_renewal(&pool, &moved).await.unwrap();

    let listed = members(&pool).await.unwrap();

    assert_eq!(listed[0].name, "studio", "and what it is now called");
    assert_eq!(
        listed[0].addresses,
        vec![
            "studio.tailnet-name.ts.net".to_owned(),
            "192.168.1.40".to_owned(),
        ],
        "in the order they were advertised, which is the order a dial works down",
    );
}

/// And a renewal about a device this membership does not hold writes nothing and
/// fails nothing.
#[tokio::test]
async fn a_renewal_about_a_stranger_changes_no_row() {
    let (_dir, pool) = fresh_pool().await;

    assert!(
        !record_renewal(&pool, &renewing(B, "workbench", B_FINGERPRINT, RENEWED))
            .await
            .unwrap(),
        "there is no row for a fingerprint to stand against",
    );

    assert!(members(&pool).await.unwrap().is_empty());
    assert!(!member_holding(&pool, RENEWED).await.unwrap());
}

/// A changeover this device has seen the end of lets go of the certificate that
/// was going out.
///
/// What says it has ended is meeting the new one, which is the only unambiguous
/// sign the far end has stopped presenting the other.
#[tokio::test]
async fn a_changeover_this_device_has_seen_the_end_of_lets_go_of_the_old_one() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();
    record_renewal(&pool, &renewing(B, "workbench", B_FINGERPRINT, RENEWED))
        .await
        .unwrap();

    changeover_over(&pool, B).await.unwrap();

    let listed = members(&pool).await.unwrap();

    assert_eq!(listed[0].fingerprint, RENEWED);
    assert_eq!(listed[0].renewing_from, None);

    assert!(
        member_holding(&pool, RENEWED).await.unwrap(),
        "the certificate it is presenting now",
    );
    assert!(
        !member_holding(&pool, B_FINGERPRINT).await.unwrap(),
        "and the one it has stopped presenting is nobody's",
    );

    // And being told twice is not a thing to fail: a member changing over from
    // nothing has already stopped.
    changeover_over(&pool, B).await.unwrap();
}

// ---------------------------------------------------------------------------
// This device's own changeover
// ---------------------------------------------------------------------------

/// Every member is owed an announcement of a certificate none of them has
/// acknowledged, and each acknowledgement takes one off that list.
///
/// **Which is what a changeover counts.** A device presents the outgoing
/// certificate for as long as this list has anybody on it, so the count is read off
/// what a member has *said* it holds rather than off the absence of a debt: a
/// device that joined in the middle of a changeover has no debt and has heard
/// nothing.
#[tokio::test]
async fn a_member_that_has_acknowledged_is_off_the_list_a_changeover_waits_on() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();
    record_member(&pool, &advertising(C, "laptop", C_FINGERPRINT))
        .await
        .unwrap();

    assert_eq!(
        owed_the_new_one(&pool).await,
        vec![C.to_owned(), B.to_owned()],
        "a member that has acknowledged nothing is owed the telling, which is \
         every member at the start of a changeover",
    );

    renewal_acknowledged(&pool, B, RENEWED).await.unwrap();

    assert_eq!(
        owed_the_new_one(&pool).await,
        vec![C.to_owned()],
        "and the one that answered is off the list",
    );

    renewal_acknowledged(&pool, C, RENEWED).await.unwrap();

    assert!(
        members_yet_to_acknowledge(&pool, RENEWED)
            .await
            .unwrap()
            .is_empty(),
        "nobody owed is the changeover over",
    );

    // And a member holding the certificate before the one coming in is owed the
    // next renewal without anything having to be cleared first.
    assert_eq!(
        owed_the_new_one_after(&pool, "11:22:33:44:55:66:77:88").await,
        vec![C.to_owned(), B.to_owned()],
        "the acknowledgement names a certificate rather than being a flag, so \
         the renewal after this one starts with everybody owed",
    );
}

/// And a member that has acknowledged is still owed the *rest* of what it missed:
/// the acknowledgement is a column on the row, and the debts are their own record.
#[tokio::test]
async fn an_acknowledged_certificate_is_not_a_debt_paid() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    owe_announcement(&pool, B, A, Telling::Joined)
        .await
        .unwrap();
    renewal_acknowledged(&pool, B, RENEWED).await.unwrap();

    assert_eq!(
        announcements_owed_to(&pool, B).await.unwrap(),
        vec![(A.to_owned(), Telling::Joined)],
        "what a member holds of this device and what it has yet to be told are \
         two different questions about it",
    );
}

/// And an unlinked member is off both: it has nothing to acknowledge and nothing
/// left to be owed.
#[tokio::test]
async fn an_unlinked_member_is_owed_no_certificate() {
    let (_dir, pool) = fresh_pool().await;

    record_member(&pool, &advertising(B, "workbench", B_FINGERPRINT))
        .await
        .unwrap();

    forget_member(&pool, B).await.unwrap();

    assert!(
        members_yet_to_acknowledge(&pool, RENEWED)
            .await
            .unwrap()
            .is_empty(),
        "a device that is not a member is a device this one owes nothing — which \
         is how a changeover finishes when the machine that never answered is \
         the one the human unlinks",
    );
}
