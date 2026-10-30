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
//! `tests/suite/transfer.rs`. What is here is the record.

use std::collections::BTreeSet;
use std::path::PathBuf;

use sqlx::SqlitePool;
use verkstead_schema::Response;
use verkstead_store::{
    Account, Addition, AdditionState, Arrival, ArrivingPicked, Ask, Birth, Cell, Commit,
    CompanionMode, Event, Lifecycle, Memo, ProfileFacts, Renaming, STAYS_BEHIND, Settlements,
    Slice, Submission, add_to_set, arrive, ask, capture, carried_tables, companions,
    create_profile, deleted_tables, insert_response, land, load_conversation, load_response,
    queue_to_join, record_commit, register_repo, session_id, set_additions, sets_as_they_landed,
    slice, standing_previews, start_conversation, submit_response, timeline, transcript,
};

use super::owning::{Worked, a_conversations_tables, fresh_pool, owning, rows};

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

/// A slice taken while the work is still committing reads the record at one
/// moment: every commit in it comes with the Event it hangs off.
///
/// **What the round trips of the server's `tests/suite/transfer.rs` kept tripping
/// over.** A slice is a table at a time, and the session behind the work can
/// still be recording a commit while one is read — an Event and its commit
/// written together between the read of the Events and the read of the
/// commits. Read as separate moments, that slice carries the commit without
/// its Event, and the far end refuses the whole move for it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_slice_taken_while_commits_land_carries_the_event_of_every_commit_in_it() {
    let (_dir, pool) = fresh_pool().await;
    let repo = super::owning::repo(&pool).await;
    let id = start_conversation(&pool, repo, "rate-limiting", BORN_ON, None)
        .await
        .unwrap()
        .unwrap();

    let committing = tokio::spawn({
        let pool = pool.clone();

        async move {
            for n in 0..300 {
                let commit = Commit {
                    sha: format!("{n:040x}"),
                    subject: format!("feat: step {n}"),
                    files: 1,
                    insertions: 1,
                    deletions: 0,
                    summary: None,
                    repo: None,
                    merge: false,
                };

                record_commit(&pool, id, repo, &commit).await.unwrap();
            }
        }
    });

    let mut slices = 0;

    while !committing.is_finished() {
        let read = slice(&pool, id).await.unwrap();
        let events = ids_in(&read, "timeline_events", "id");
        let named = ids_in(&read, "commits", "event_id");

        assert!(
            named.is_subset(&events),
            "slice {slices} carries commits naming Events it does not carry: {:?}",
            named.difference(&events).collect::<Vec<_>>(),
        );

        slices += 1;
    }

    committing.await.unwrap();
}

/// The ids one column of one carried table holds in a slice.
fn ids_in(slice: &Slice, table: &str, column: &str) -> BTreeSet<i64> {
    let rows = slice
        .tables
        .iter()
        .find(|rows| rows.table == table)
        .unwrap_or_else(|| panic!("a slice carries {table}"));

    let Some(at) = rows.columns.iter().position(|named| named == column) else {
        // No rows read, so no columns named either.
        return BTreeSet::new();
    };

    rows.rows
        .iter()
        .filter_map(|row| match row[at] {
            Cell::Int(id) => Some(id),
            _ => None,
        })
        .collect()
}

/// A Preview crosses with its command, its description and its stamp, under
/// the id the far end gave the Conversation.
#[tokio::test]
async fn a_preview_crosses_with_its_command_description_and_stamp() {
    let moved = moved("rate-limiting").await;

    let here = standing_previews(&moved.here.pool, moved.worked.id)
        .await
        .unwrap();
    let there = standing_previews(&moved.there.pool, moved.to)
        .await
        .unwrap();

    assert_eq!(here.len(), 1, "the fixture suggested one: {here:?}");
    assert!(
        here[0].description.is_some(),
        "with a description: {here:?}"
    );
    assert_eq!(there, here, "and it crossed as it stood");
}

/// A Memo crosses with its title and its stamp, and the session it names is the one that
/// landed over there rather than whichever Event happens to hold that number.
#[tokio::test]
async fn a_memo_crosses_stamped_and_naming_the_session_that_left_it() {
    let moved = moved("rate-limiting").await;

    let (here, left_here) = memo(&moved.here.pool, moved.worked.id).await;
    let (there, left_there) = memo(&moved.there.pool, moved.to).await;

    assert_eq!(there, here, "the markdown crossed as it was written");
    assert_eq!(
        left_there.title,
        Some("The sessions suite flakes".to_owned()),
        "its title with it"
    );
    assert_eq!(
        left_there.left_by, left_here.left_by,
        "and its stamp with it"
    );
    assert_eq!(left_there.retired, None);

    // Where the session that left it sits on each Timeline, counted from the
    // top: the ids are each database's own, and the position is the record.
    let at = |events: &[i64], id: i64| events.iter().position(|event| *event == id);
    let was = at(&moved.events, left_here.left_in);

    assert!(was.is_some(), "the Memo names an Event of its own Timeline");
    assert_eq!(
        at(&landed(&moved.there.pool).await, left_there.left_in),
        was,
        "and it names the session that landed over there",
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

/// The Timeline read while the record lands again reads one record or the
/// other, never the Events of the first with the Captures of the second.
///
/// **What the round trips of the server's `tests/suite/transfer.rs` tripped over once
/// the slice was read at one moment.** A return lands against the record
/// already there, taking it away and writing every Event under a new id in one
/// transaction — and the Timeline is a query for the Events and one more for
/// each kind joined in beside them. Read across that landing, its Events are
/// the old ones and its Captures the new, so every session's output on it has
/// none, which fails the whole Conversation's page.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_timeline_read_while_the_record_lands_again_reads_one_record() {
    let moved = moved("rate-limiting").await;

    let read = slice(&moved.here.pool, moved.worked.id).await.unwrap();

    let renaming = Renaming {
        repos: repos(&moved.here.pool)
            .await
            .into_iter()
            .zip(repos(&moved.there.pool).await)
            .collect(),
        ..Renaming::default()
    };

    let landing = tokio::spawn({
        let pool = moved.there.pool.clone();
        let to = moved.to;

        async move {
            for _ in 0..30 {
                land(&pool, to, &read, &renaming).await.unwrap();
            }
        }
    });

    let mut reads = 0;

    while !landing.is_finished() {
        timeline(&moved.there.pool, moved.to)
            .await
            .unwrap_or_else(|why| panic!("read {reads} of the Timeline failed: {why:#}"));

        reads += 1;
    }

    landing.await.unwrap();
}

/// A stage that crosses queued to join its roadmap's chain arrives queued, at a
/// place the far end issues — behind whatever is already waiting there.
///
/// The place is an order rather than a fact, and every Verkstead counts its own:
/// one carried as the number it was would land in front of stages that finished
/// over there before it arrived, or on a place already taken.
#[tokio::test]
async fn a_stage_queued_to_join_lands_queued_behind_what_waits_there() {
    let moved = moved("rate-limiting").await;

    // The copy on the far end held no place of its own: it was handed on before
    // its tasks there had finished. One that did keeps it, which is the reports
    // suite's: a stage coming home.
    sqlx::query("DELETE FROM stage_joinings WHERE conversation_id = ?")
        .bind(moved.to)
        .execute(&moved.there.pool)
        .await
        .unwrap();

    // Another stage already waiting on the far end, queued before the record
    // lands again — which is what a return over a busy device is.
    let theirs = repos(&moved.there.pool).await;
    let waiting = start_conversation(&moved.there.pool, theirs[0], "waiting", BORN_ON, None)
        .await
        .unwrap()
        .expect("the Repo is registered");

    queue_to_join(&moved.there.pool, waiting).await.unwrap();

    let read = slice(&moved.here.pool, moved.worked.id).await.unwrap();

    let renaming = Renaming {
        repos: repos(&moved.here.pool)
            .await
            .into_iter()
            .zip(theirs)
            .collect(),
        ..Renaming::default()
    };

    land(&moved.there.pool, moved.to, &read, &renaming)
        .await
        .unwrap();

    let place = |conversation: i64| {
        let pool = moved.there.pool.clone();

        async move {
            sqlx::query_as::<_, (i64,)>(
                "SELECT place FROM stage_joinings WHERE conversation_id = ?",
            )
            .bind(conversation)
            .fetch_optional(&pool)
            .await
            .unwrap()
            .map(|(place,)| place)
        }
    };

    let landed = place(moved.to)
        .await
        .expect("the stage crossed with its place in the queue");
    let before = place(waiting)
        .await
        .expect("and the other one is still queued");

    assert!(
        landed > before,
        "it waits behind the stage already waiting there: {landed} against {before}",
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
/// `continuations::sets_as_they_landed`, and the server's own `tests/suite/transfer.rs`
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

/// What was picked on a Set nobody has answered crosses with it (ADR-0028,
/// *Pending additions cross with the record*): a companion of a repository the
/// far end holds lands naming that end's Repo, still pending, and the Answer
/// there puts it on the Conversation; one of a repository it holds none of
/// lands struck, under the name it had, rather than refusing the move; and a
/// device tick lands as it was.
#[tokio::test]
async fn a_sets_pending_picks_cross_and_a_repo_with_no_match_is_struck() {
    let here = end().await;
    let worked = owning(&here.pool, "rate-limiting").await;
    let open = ask(
        &here.pool,
        worked.id,
        &super::owning::asked(),
        Ask::Blocking,
    )
    .await
    .unwrap()
    .expect("the Conversation is there to ask from")
    .id;

    let ours = repos(&here.pool).await;
    let shared = register_repo(
        &here.pool,
        &PathBuf::from("/watched/shared"),
        "shared",
        "main",
    )
    .await
    .unwrap()
    .unwrap()
    .id;
    let lonely = register_repo(
        &here.pool,
        &PathBuf::from("/watched/lonely"),
        "lonely",
        "main",
    )
    .await
    .unwrap()
    .unwrap()
    .id;

    for addition in [
        Addition::Companion {
            repo_id: shared,
            mode: CompanionMode::ReadOnly,
            base: None,
            branch: None,
        },
        Addition::Companion {
            repo_id: lonely,
            mode: CompanionMode::ReadWrite,
            base: None,
            branch: Some("lonely-work".to_owned()),
        },
        Addition::Device {
            device: BORN_ON.to_owned(),
        },
    ] {
        add_to_set(&here.pool, worked.id, open, Some("Q1"), &addition)
            .await
            .unwrap();
    }

    let mut read = slice(&here.pool, worked.id).await.unwrap();

    assert!(
        !read.repos().contains(&shared) && !read.repos().contains(&lonely),
        "a Repo only a pending pick names is not one the move needs",
    );
    assert_eq!(read.picked_repos(), BTreeSet::from([shared, lonely]));

    // The far end holds the two the fixture works in and `shared`, and nothing
    // that is `lonely`: the sending device strikes it, as `transfers` does.
    read.strike(lonely, "lonely");

    let there = end().await;
    let theirs = repos(&there.pool).await;
    let shared_there = register_repo(&there.pool, &PathBuf::from("/far/shared"), "shared", "main")
        .await
        .unwrap()
        .unwrap()
        .id;
    let to = arrived(&there.pool, theirs[0]).await;

    let renaming = Renaming {
        repos: ours
            .into_iter()
            .zip(theirs)
            .chain([(shared, shared_there)])
            .collect(),
        ..Renaming::default()
    };

    land(&there.pool, to, &read, &renaming).await.unwrap();

    let landed = *sets_as_they_landed(&there.pool, to)
        .await
        .unwrap()
        .get(&open)
        .expect("the open Set crossed");

    let picked: Vec<(Addition, AdditionState)> = set_additions(&there.pool, landed)
        .await
        .unwrap()
        .into_iter()
        .map(|added| (added.addition, added.state))
        .collect();

    assert_eq!(
        picked,
        vec![
            (
                Addition::Companion {
                    repo_id: shared_there,
                    mode: CompanionMode::ReadOnly,
                    base: None,
                    branch: None,
                },
                AdditionState::Pending,
            ),
            (
                Addition::Unmatched {
                    name: "lonely".to_owned(),
                },
                AdditionState::Struck,
            ),
            (
                Addition::Device {
                    device: BORN_ON.to_owned(),
                },
                AdditionState::Pending,
            ),
        ],
    );

    // And answered there, what survived is put on the Conversation there.
    insert_response(&there.pool, landed, &Response::default())
        .await
        .unwrap()
        .expect("the Set is open there");

    let companions = companions(&there.pool, to).await.unwrap();

    assert!(
        companions
            .iter()
            .any(|companion| companion.repo.id == shared_there),
        "the companion picked before the move is the far end's now",
    );
    assert!(
        companions
            .iter()
            .all(|companion| companion.repo.name != "lonely"),
        "and the struck one is not",
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
    ask(
        &here.pool,
        worked.id,
        &super::owning::asked(),
        Ask::Blocking,
    )
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

/// The first Memo on a Timeline, with what was written down beside it.
async fn memo(pool: &SqlitePool, id: i64) -> (String, Memo) {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .find_map(|event| match event.event {
            Event::Memo(markdown, memo) => Some((markdown, memo)),
            _ => None,
        })
        .expect("the fixture left a Memo")
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
