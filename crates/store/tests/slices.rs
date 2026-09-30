//! The **slice** of the record one Conversation is, read off one database and
//! written into another (ADR-0020, *Transfer*).
//!
//! Two databases really, because that is what a move is: everything the store
//! keeps about one Conversation goes out of the first and lands in the second
//! under ids the second issued. So what is worth asserting is what no read on
//! the sending side could notice — that every table crossed, that every id came
//! out pointing at what it landed as, and that a Set left open is still a Set
//! anybody can answer.
//!
//! The coverage is read off the schema rather than written down here. The
//! fixture fills every table the schema says is a Conversation's — see
//! [`owning`] — and the manifest has to account for every one of them, either by
//! carrying it or by naming it as the device's own. A table added next year is a
//! failing test here until somebody has decided which it is.
//!
//! What the two ends make of this over a real link is the server's
//! `tests/transfer.rs`. What is here is the record.

use std::collections::BTreeSet;
use std::path::PathBuf;

use sqlx::SqlitePool;
use verkstead_schema::Response;
use verkstead_store::{
    Account, Arrival, ArrivingPicked, Ask, Birth, Lifecycle, ProfileFacts, Renaming, STAYS_BEHIND,
    Settlements, Submission, arrive, ask, capture, carried_tables, create_profile, deleted_tables,
    land, load_conversation, load_response, register_repo, session_id, sets_as_they_landed, slice,
    submit_response, timeline, transcript,
};

mod owning;

use owning::{Worked, a_conversations_tables, fresh_pool, owning, rows};

/// The device the work was born and worked on, which is neither of the two
/// databases here — a Conversation that has moved twice still answers to the key
/// it was born under.
const BORN_ON: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// The Rank it sits at, carrying the device that issued it.
const RANK: &str = "a0-aa00bb11cc22dd33ee44ff5566778899";

/// The two repositories the fixture works in, by the names both machines call
/// them.
const REPOSITORIES: [&str; 2] = ["verkstead", "askance"];

/// Every table a Conversation's rows live in is one a move has decided about: it
/// either travels with the work or is named as the device's own.
///
/// **The one test here that outlives the rest.** Verkstead keeps a Conversation
/// in two dozen tables, declared a module at a time, so the thing most likely to
/// go wrong is not the walk being wrong today but a table added next year that
/// nobody carried — a Timeline arriving without whatever that table held, and
/// nothing anywhere to say it had. Read off the schema, and off the delete's own
/// list, which is held against the schema for the same reason.
#[tokio::test]
async fn every_table_of_a_conversations_is_carried_or_named_as_staying() {
    let (_dir, pool) = fresh_pool().await;
    owning(&pool, "rate-limiting").await;

    let theirs = a_conversations_tables(&pool).await;
    let walked: BTreeSet<String> = deleted_tables().into_iter().map(str::to_owned).collect();

    let carried: BTreeSet<String> = carried_tables().into_iter().map(str::to_owned).collect();
    let stays: BTreeSet<String> = STAYS_BEHIND
        .iter()
        .map(|table| (*table).to_owned())
        .collect();

    let undecided: Vec<&String> = theirs
        .union(&walked)
        .filter(|table| !carried.contains(*table) && !stays.contains(*table))
        .collect();

    assert!(
        undecided.is_empty(),
        "these tables hold a Conversation's rows and a move says nothing about \
         them: {undecided:?}",
    );

    for table in &carried {
        assert!(
            rows(&pool, table).await > 0,
            "the fixture leaves nothing in {table}, so a slice carrying it is \
             something this suite never sees happen",
        );
    }

    // And the clearing a landing record does in front of itself is over exactly
    // those tables: a record lands over whatever was here, so a table a slice
    // carries and the clearing does not would be rows of the old copy left
    // under the new one — and one the clearing takes and a slice does not would
    // be a device's own emptied by somebody else's move.
    let cleared: BTreeSet<String> = verkstead_store::cleared_tables()
        .into_iter()
        .map(str::to_owned)
        .collect();

    assert_eq!(
        cleared, carried,
        "what a record landing takes out is what a record carries",
    );
}

/// The whole record crosses, renumbered: every table lands with what it held,
/// and the Timeline on the far end reads as it read here.
///
/// The Events in the same order drawing the same cards, and behind each of them
/// the Capture, the Transcript and the name the session ran under — which are
/// the rows most likely to be left pointing at an Event of the wrong database,
/// being the ones keyed by an Event rather than by the Conversation.
#[tokio::test]
async fn the_record_lands_on_the_far_end_reading_as_it_read_here() {
    let moved = moved("rate-limiting").await;

    for table in carried_tables() {
        assert_eq!(
            rows(&moved.there.pool, table).await,
            rows(&moved.here.pool, table).await,
            "{table} did not cross whole",
        );
    }

    assert_eq!(
        events(&moved.there.pool).await,
        events(&moved.here.pool).await,
        "the Timeline reads as it did: the same Events, in the same order, each \
         drawing the card it drew",
    );

    assert_eq!(
        timeline(&moved.there.pool, moved.to).await.unwrap().len(),
        timeline(&moved.here.pool, moved.worked.id)
            .await
            .unwrap()
            .len(),
        "and it draws as many cards over there as it did here",
    );

    // Behind the Events: what the two sessions printed, the logs they kept of
    // themselves and the names they ran under — every one of them keyed by an
    // Event rather than by the Conversation.
    let there = landed(&moved.there.pool).await;

    assert_eq!(
        there.len(),
        moved.events.len(),
        "every Event crossed, which is what the rest of this is read by",
    );

    for (was, now) in moved.events.iter().zip(there) {
        assert_eq!(
            capture(&moved.there.pool, moved.to, now).await.unwrap(),
            capture(&moved.here.pool, moved.worked.id, *was)
                .await
                .unwrap(),
            "the Capture behind Event {was}",
        );
        assert_eq!(
            transcript(&moved.there.pool, moved.to, now).await.unwrap(),
            transcript(&moved.here.pool, moved.worked.id, *was)
                .await
                .unwrap(),
            "the Transcript behind Event {was}",
        );
        assert_eq!(
            session_id(&moved.there.pool, now).await.unwrap(),
            session_id(&moved.here.pool, *was).await.unwrap(),
            "and the name the session behind Event {was} ran under",
        );
    }

    // And the stop, which is the one thing about a Conversation that points the
    // other way: a column on the row naming an Event on its own Timeline.
    let (notice,): (Option<i64>,) =
        sqlx::query_as("SELECT stopped_notice FROM conversations WHERE id = ?")
            .bind(moved.to)
            .fetch_one(&moved.there.pool)
            .await
            .unwrap();

    let notice = notice.expect("the stop crossed with the work");

    assert!(
        landed(&moved.there.pool).await.contains(&notice),
        "and its Notice names an Event of this database's own",
    );
}

/// **And a record lands over the one that was already here**, which is what makes
/// a Conversation coming home one Conversation rather than two records in one.
///
/// The copy a device kept when it handed the work on is stale by the time the
/// work comes back, and the record arriving is the live one: there is nothing to
/// merge and nothing to reconcile, so what was here goes in the very transaction
/// the new rows land in. Landed twice over, the far end would hold two of every
/// card on the Timeline and two of every Set behind them.
#[tokio::test]
async fn a_record_landing_again_takes_the_one_that_was_here_with_it() {
    let moved = moved("rate-limiting").await;

    let first = landed(&moved.there.pool).await;

    // The same record again, which is what a return is: the device the work went
    // to sending it back to the one it came from, against the row that is
    // already there.
    let read = slice(&moved.here.pool, moved.worked.id).await.unwrap();

    let renaming = Renaming {
        repos: repos(&moved.here.pool)
            .await
            .into_iter()
            .zip(repos(&moved.there.pool).await)
            .collect(),
        ..Renaming::default()
    };

    land(&moved.there.pool, moved.to, &read, &renaming)
        .await
        .unwrap();

    for table in carried_tables() {
        assert_eq!(
            rows(&moved.there.pool, table).await,
            rows(&moved.here.pool, table).await,
            "{table} holds what the record holds rather than two landings of it",
        );
    }

    assert_eq!(
        events(&moved.there.pool).await,
        events(&moved.here.pool).await,
        "and the Timeline reads once through rather than twice",
    );

    let second = landed(&moved.there.pool).await;

    assert!(
        second.iter().all(|event| !first.contains(event)),
        "every Event on it is one this landing wrote: {second:?} against {first:?}",
    );
}

/// A Question Set left open crosses answerable, and its Answer lands where the
/// work now is.
///
/// Which is the whole of what moving a Set is for. One whose id had not moved
/// with it would be answered into a Conversation of somebody else's, and one
/// that arrived already settled would be a question the human could no longer
/// answer at all.
#[tokio::test]
async fn a_set_left_open_is_answerable_where_the_work_now_is() {
    let moved = moved("rate-limiting").await;

    assert_eq!(
        asked_from(&moved.there.pool, moved.to).await.len(),
        asked_from(&moved.here.pool, moved.worked.id).await.len(),
        "every Set asked from it crossed",
    );

    let open = *open_sets(&moved.there.pool)
        .await
        .first()
        .expect("the Conversation crossed with a Set nobody has answered");

    assert!(
        asked_from(&moved.there.pool, moved.to)
            .await
            .contains(&open),
        "and it is one of this Conversation's, by an id of this database's own",
    );

    let accepted = submit_response(
        &moved.there.pool,
        &Settlements::new(4),
        open,
        &Response::default(),
    )
    .await
    .unwrap();

    assert!(
        matches!(accepted, Submission::Accepted(_)),
        "it takes an Answer, which is what reaches a session waiting here: \
         {accepted:?}",
    );

    assert!(
        load_response(&moved.there.pool, open)
            .await
            .unwrap()
            .is_some(),
        "and the record on the far end now holds it",
    );

    // While the Set that was answered before the move arrived answered: a
    // Response is the record of a decision, and a copy that lost it would be one
    // asking the human again.
    assert!(
        open_sets(&moved.here.pool).await.len() == open_sets(&moved.there.pool).await.len() + 1,
        "the one just answered is the only one either end has settled since",
    );
}

/// And what each Set landed *as* is written down against the Conversation, which
/// is the one piece of the landing's arithmetic anything after it needs.
///
/// A resumed agent knows its Questions by the ids it asked them under, and those
/// are the sending device's. The landing builds the map because everything pointing
/// at a Set has to be renumbered against it, and the record lands in a leg of its
/// own — one before the session that resumes is started — so a map dropped at the
/// commit would be a note that could not say which question was which. See
/// `continuations::sets_as_they_landed`, and the server's own `tests/transfer.rs`
/// for the note it turns into.
///
/// **And a second landing writes its own over it.** A Conversation that has come
/// and gone and come back again landed under ids of its own each time, and the map
/// of two moves ago names Sets this device has renumbered since.
#[tokio::test]
async fn what_each_set_landed_as_is_written_down_against_the_conversation() {
    let moved = moved("rate-limiting").await;

    let there = asked_from(&moved.there.pool, moved.to).await;

    assert_eq!(
        sets_as_they_landed(&moved.there.pool, moved.to)
            .await
            .unwrap(),
        asked_from(&moved.here.pool, moved.worked.id)
            .await
            .into_iter()
            .zip(there.iter().copied())
            .collect(),
        "every Set of the record is there, by the id it was asked under against \
         the id it landed as",
    );

    // The same record again, which is what a return is — and the ids it lands
    // under this time are ids nothing has issued yet.
    let read = slice(&moved.here.pool, moved.worked.id).await.unwrap();

    let renaming = Renaming {
        repos: repos(&moved.here.pool)
            .await
            .into_iter()
            .zip(repos(&moved.there.pool).await)
            .collect(),
        ..Renaming::default()
    };

    land(&moved.there.pool, moved.to, &read, &renaming)
        .await
        .unwrap();

    let again = asked_from(&moved.there.pool, moved.to).await;

    // The Sets are issued from a sequence rather than from whatever the table has
    // room for — see `question_sets` — so the second landing's ids are ids nothing
    // has ever held, and the map is read against numbers that are not the ones it
    // was built from. Which is what makes the rest of this a test of the
    // arithmetic rather than of two databases that happened to count alike.
    assert!(
        again.iter().all(|landed| !there.contains(landed)),
        "the second landing issued ids of its own: {again:?} against {there:?}",
    );

    assert_eq!(
        sets_as_they_landed(&moved.there.pool, moved.to)
            .await
            .unwrap(),
        asked_from(&moved.here.pool, moved.worked.id)
            .await
            .into_iter()
            .zip(again)
            .collect(),
        "and the map is that landing's rather than the one before it: one row per \
         Set, each naming what this landing issued",
    );
}

/// The two ids a slice cannot renumber for itself: a Repo with nowhere to land
/// refuses the whole record, and an Agent Profile the far end has never heard of
/// leaves its column empty.
///
/// Different answers because they are different stakes. A row landing under the
/// wrong repository is work put in the wrong directory, which is the failure the
/// whole matching rule exists to prevent; a Steer whose account was deleted a
/// year later is history with a name missing from it, and the rest of the row is
/// still the record.
#[tokio::test]
async fn a_repository_with_nowhere_to_land_refuses_and_a_forgotten_account_does_not() {
    let moved = moved("rate-limiting").await;

    let (profile, digest): (Option<i64>, i64) =
        sqlx::query_as("SELECT profile_id, digest FROM steers WHERE conversation_id = ?")
            .bind(moved.to)
            .fetch_one(&moved.there.pool)
            .await
            .unwrap();

    assert_eq!(
        profile, None,
        "an account this device has never heard of is a name the record cannot \
         keep",
    );
    assert_eq!(digest, 1, "and the rest of the Steer crossed as it stood");

    // And a record whose repositories a device holds none of is refused whole,
    // rather than landing rows under whichever repository happened to be there.
    let bare = end().await;
    let alone = arrived(&bare.pool, repos(&bare.pool).await[0]).await;

    let refused = land(
        &bare.pool,
        alone,
        &slice(&moved.here.pool, moved.worked.id).await.unwrap(),
        &Renaming::default(),
    )
    .await;

    let said = format!(
        "{:#}",
        refused.expect_err("no Repo here is that repository")
    );

    assert!(
        said.contains("no Repo on this device is"),
        "the refusal says a repository had nowhere to land: {said}",
    );
}

/// One database, held open for the length of a test.
struct End {
    _dir: tempfile::TempDir,
    pool: SqlitePool,
}

/// A fresh one.
async fn end() -> End {
    let (dir, pool) = fresh_pool().await;

    End { _dir: dir, pool }
}

/// One Conversation moved between two of them, and the handles both ends are
/// read by.
struct Moved {
    here: End,
    there: End,

    /// What was worked on the device it came off.
    worked: Worked,

    /// Its Timeline Events there, in the order they were written — which is the
    /// order they land in over here.
    events: Vec<i64>,

    /// And the id the far end gave its copy.
    to: i64,
}

/// Work one Conversation the whole way on one database, and move its record onto
/// another.
async fn moved(branch: &str) -> Moved {
    let here = end().await;
    let worked = owning(&here.pool, branch).await;

    // A Steer run under an account of that device's, so that the one id a slice
    // is allowed to lose has something to lose.
    let account = create_profile(
        &here.pool,
        &ProfileFacts {
            name: Some("steered".to_owned()),
            account: Account::Codex {
                home: PathBuf::from("/watched/accounts/steered/.codex"),
            },
            models: vec!["gpt-5".to_owned()],
            memory: true,
        },
    )
    .await
    .unwrap()
    .expect("nothing is called that yet")
    .id;

    sqlx::query("UPDATE steers SET profile_id = ?, digest = 1 WHERE conversation_id = ?")
        .bind(account)
        .bind(worked.id)
        .execute(&here.pool)
        .await
        .unwrap();

    // And a Set nobody has answered, which is what a move has to leave
    // answerable on the far end — the fixture's own were all settled by the
    // close.
    ask(&here.pool, worked.id, &owning::asked(), Ask::Blocking)
        .await
        .unwrap()
        .expect("the Conversation is there to ask from");

    let events = landed(&here.pool).await;
    let read = slice(&here.pool, worked.id).await.unwrap();

    // The far end: the same two repositories under ids of its own, and the
    // Conversation's row written the way a transfer writes one.
    let there = end().await;
    let theirs = repos(&there.pool).await;
    let to = arrived(&there.pool, theirs[0]).await;

    let ours = repos(&here.pool).await;

    let renaming = Renaming {
        repos: ours.into_iter().zip(theirs).collect(),
        ..Renaming::default()
    };

    land(&there.pool, to, &read, &renaming).await.unwrap();

    assert!(
        load_conversation(&there.pool, to).await.unwrap().is_some(),
        "the Conversation is there to have landed against",
    );

    Moved {
        here,
        there,
        worked,
        events,
        to,
    }
}

/// The two repositories, registered under whatever ids this database issues.
async fn repos(pool: &SqlitePool) -> Vec<i64> {
    let mut registered = Vec::new();

    for name in REPOSITORIES {
        register_repo(pool, &PathBuf::from("/watched").join(name), name, "main")
            .await
            .unwrap();

        let (id,): (i64,) = sqlx::query_as("SELECT id FROM repos WHERE name = ?")
            .bind(name)
            .fetch_one(pool)
            .await
            .unwrap();

        registered.push(id);
    }

    registered
}

/// The Conversation's own row on the far end, written the way a transfer writes
/// one — which is what a slice lands beside.
async fn arrived(pool: &SqlitePool, repo_id: i64) -> i64 {
    arrive(
        pool,
        &Arrival {
            born: Birth {
                device: BORN_ON.to_owned(),
                id: 7,
            },
            repo_id,
            branch: "rate-limiting".to_owned(),
            branch_named: true,
            naming: false,
            state: Lifecycle::Implementing,
            rank: RANK.to_owned(),
            grilling: ArrivingPicked::Nothing,
            implementation: ArrivingPicked::Nothing,
            review: ArrivingPicked::Skipped,
        },
    )
    .await
    .unwrap()
    .expect("the Repo is registered")
}

/// One database's Timeline Events, in the order they were written.
async fn landed(pool: &SqlitePool) -> Vec<i64> {
    let rows: Vec<(i64,)> = sqlx::query_as("SELECT id FROM timeline_events ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap();

    rows.into_iter().map(|(id,)| id).collect()
}

/// And what each of them says, which is what a Timeline *is*: the ids are each
/// database's own and the cards are the record.
async fn events(pool: &SqlitePool) -> Vec<(String, String)> {
    sqlx::query_as("SELECT kind, body FROM timeline_events ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The Question Sets asked from one Conversation, by that database's own ids.
async fn asked_from(pool: &SqlitePool, conversation_id: i64) -> Vec<i64> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT set_id FROM set_events
         WHERE event_id IN (SELECT id FROM timeline_events WHERE conversation_id = ?)
         ORDER BY set_id",
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    .unwrap();

    rows.into_iter().map(|(id,)| id).collect()
}

/// And the ones nobody has settled: no Response, and no lock either.
async fn open_sets(pool: &SqlitePool) -> Vec<i64> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT q.id FROM question_sets q
         LEFT JOIN responses r ON r.set_id = q.id
         LEFT JOIN archivings a ON a.set_id = q.id
         WHERE r.set_id IS NULL AND a.set_id IS NULL
         ORDER BY q.id",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    rows.into_iter().map(|(id,)| id).collect()
}
