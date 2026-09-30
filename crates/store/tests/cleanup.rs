//! What a Cleanup takes out of an archived Conversation, what it leaves behind,
//! and what is left when it takes the whole of it.
//!
//! The rule is the card: what a Timeline card draws survives a trim, and what
//! only a drill-down shows does not. So the Timeline itself is what most of
//! these assert against — read whole before the trim and read whole after it,
//! because every card-feeding row there is hangs off one of those Events, and a
//! trim that took one of them would show up as a Timeline that had changed.
//!
//! The other half is the clock. It runs from the archiving rather than from the
//! Conversation, so what is worth a test is what a promise could not keep: that
//! a fresh archiving is left alone, that an unarchived Conversation has no clock
//! at all, and that one archived a second time has its new bulk taken as well as
//! its old.
//!
//! A delete has no boundary to draw and so asserts the other way about: that
//! nothing is left anywhere. What *anywhere* is comes out of the schema rather
//! than out of a list written here — see [`owning::a_conversations_tables`],
//! which is what makes this a test a table added next year has to answer to.
//!
//! The Conversation with one of everything on it lives in [`owning`], beside
//! that reading, because a **transfer's slice** has to be held against the same
//! set of tables from the other end — see the `slices` suite.

mod owning;

use std::collections::BTreeSet;

use sqlx::SqlitePool;
use verkstead_store::{
    Deletion, Summary, Trimming, append_capture, archive_conversation, capture, deletable,
    delete_conversation, deleted_tables, load_conversation, load_response, reclaim, session_id,
    start_capture, timeline, transcript, trim_conversation, trimmable, trimmed,
    unarchive_conversation,
};

use owning::{a_conversations_tables, fresh_pool, owning, printed, rows, worked};

/// Put an archiving back in time, which is the only way a test gets to be days
/// old.
async fn archived_days_ago(pool: &SqlitePool, id: i64, days: u32) {
    sqlx::query(
        "UPDATE archived_conversations
         SET archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?)
         WHERE conversation_id = ?",
    )
    .bind(format!("-{days} days"))
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

/// And a trim mark, for the Conversation whose last trim was a life ago.
async fn trimmed_days_ago(pool: &SqlitePool, id: i64, days: u32) {
    sqlx::query(
        "UPDATE trimmed_conversations
         SET trimmed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?)
         WHERE conversation_id = ?",
    )
    .bind(format!("-{days} days"))
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

/// A trim takes the bulk and nothing else: the Capture's chunks, the Transcript
/// and the session's name go, and every card on the Timeline is where it was.
///
/// The Timeline read whole on both sides is the assertion that matters. The
/// summary an agent-output card draws, the Set and how it was settled, the
/// commit and its summary, the pull request — all of them are read through it,
/// so a trim that reached one of them could not leave it equal.
#[tokio::test]
async fn trimming_takes_the_bulk_and_leaves_every_card() {
    let (_dir, pool) = fresh_pool().await;
    let worked = worked(&pool, "rate-limiting").await;

    let before = timeline(&pool, worked.id).await.unwrap();

    assert_eq!(
        trim_conversation(&pool, worked.id).await.unwrap(),
        Trimming::Trimmed
    );

    assert_eq!(
        capture(&pool, worked.id, worked.event).await.unwrap(),
        Some(String::new()),
        "the Capture's Event is still on the Timeline and there is nothing in it",
    );
    assert_eq!(
        transcript(&pool, worked.id, worked.event).await.unwrap(),
        Some(Vec::new()),
        "and the log the session kept of itself is gone line for line",
    );
    assert_eq!(
        session_id(&pool, worked.event).await.unwrap(),
        None,
        "and the name it ran under, which was only ever a log to look up",
    );

    assert_eq!(
        timeline(&pool, worked.id).await.unwrap(),
        before,
        "while every card is exactly as it was: the summary the output card \
         draws, the Set, the commit and its summary, the pull request, and what \
         the session ran under",
    );

    assert!(
        load_response(&pool, worked.set).await.unwrap().is_some(),
        "and the Answer the human gave, which is the record of a decision",
    );
}

/// Trimming one that has been trimmed since it was archived is nothing
/// happening, rather than a second pass over rows that have gone.
#[tokio::test]
async fn trimming_again_finds_nothing_left_to_take() {
    let (_dir, pool) = fresh_pool().await;
    let worked = worked(&pool, "rate-limiting").await;

    assert_eq!(
        trim_conversation(&pool, worked.id).await.unwrap(),
        Trimming::Trimmed
    );
    assert_eq!(
        trim_conversation(&pool, worked.id).await.unwrap(),
        Trimming::AlreadyTrimmed
    );
}

/// A Conversation nobody archived is refused: the archiving is what authorises
/// the loss, and there is nothing else in the record that does.
#[tokio::test]
async fn what_was_never_archived_is_not_trimmed() {
    let (_dir, pool) = fresh_pool().await;
    let worked = worked(&pool, "rate-limiting").await;

    unarchive_conversation(&pool, worked.id).await.unwrap();

    assert_eq!(
        trim_conversation(&pool, worked.id).await.unwrap(),
        Trimming::NotArchived
    );

    assert_eq!(
        capture(&pool, worked.id, worked.event)
            .await
            .unwrap()
            .as_deref(),
        Some("the session said a great deal\n"),
        "and the bulk is untouched, a refusal being a refusal",
    );

    assert_eq!(
        trim_conversation(&pool, 404).await.unwrap(),
        Trimming::NoSuchConversation
    );
}

/// What is there to trim is what has been archived for longer than the days:
/// not a fresh archiving, and not one the human has taken back out.
#[tokio::test]
async fn what_is_trimmable_is_what_has_been_archived_for_long_enough() {
    let (_dir, pool) = fresh_pool().await;

    let old = worked(&pool, "rate-limiting").await;
    archived_days_ago(&pool, old.id, 4).await;

    let fresh = worked(&pool, "usage-limits").await;

    let back = worked(&pool, "window-rollover").await;
    archived_days_ago(&pool, back.id, 4).await;
    unarchive_conversation(&pool, back.id).await.unwrap();

    let waiting = trimmable(&pool, 3).await.unwrap();

    assert!(
        !waiting.contains(&fresh.id),
        "one archived a moment ago is not old enough to have anything taken",
    );
    assert!(
        !waiting.contains(&back.id),
        "and one the human has taken back out has no clock running at all",
    );
    assert_eq!(
        waiting,
        [old.id],
        "so what is left is the one archived four days ago",
    );

    trim_conversation(&pool, old.id).await.unwrap();

    assert!(
        trimmable(&pool, 3).await.unwrap().is_empty(),
        "and once it has been trimmed there is nothing left to do at all",
    );
}

/// A fresh archiving starts the clock again, so a Conversation steered back to
/// life and put away a second time has its new bulk taken too.
///
/// The mark says when it was last trimmed rather than that it ever was, which is
/// what makes this a comparison rather than a flag nothing could clear. And the
/// mark stays where it is through the unarchiving in the middle: what was taken
/// is gone whatever happens next.
#[tokio::test]
async fn a_conversation_archived_again_is_trimmable_again() {
    let (_dir, pool) = fresh_pool().await;

    let worked = worked(&pool, "rate-limiting").await;
    archived_days_ago(&pool, worked.id, 10).await;
    trim_conversation(&pool, worked.id).await.unwrap();
    trimmed_days_ago(&pool, worked.id, 9).await;

    // Back on the list, worked on again, and put away again — which is the whole
    // of what makes it trimmable a second time.
    unarchive_conversation(&pool, worked.id).await.unwrap();
    let again = printed(
        &pool,
        worked.id,
        "second-session",
        "and said a great deal more",
    )
    .await;
    archive_conversation(&pool, worked.id).await.unwrap();
    archived_days_ago(&pool, worked.id, 4).await;

    assert_eq!(
        trimmable(&pool, 3).await.unwrap(),
        [worked.id],
        "the trim it carries is older than the archiving it is under now",
    );

    assert_eq!(
        trim_conversation(&pool, worked.id).await.unwrap(),
        Trimming::Trimmed
    );

    assert_eq!(
        capture(&pool, worked.id, again).await.unwrap(),
        Some(String::new()),
        "and it is the second session's output that has been taken",
    );
}

/// The mark the Conversation's page reads, which stays where it is once it has
/// been written.
///
/// Deliberately not the sweep's rule read backwards. The sweep asks whether
/// there is a trim to *do*, which a fresh archiving makes true again; this asks
/// whether a trim has been *done*, which is what a page has to know to explain
/// a session's missing drill-down — and that is true from the trim onwards, an
/// unarchiving and a second archiving included. What was taken is gone whatever
/// the Conversation does next.
#[tokio::test]
async fn the_trimmed_mark_outlasts_the_clock_it_was_written_under() {
    let (_dir, pool) = fresh_pool().await;

    let worked = worked(&pool, "rate-limiting").await;
    archived_days_ago(&pool, worked.id, 10).await;

    assert!(
        !trimmed(&pool, worked.id).await.unwrap(),
        "nothing has been taken out of it yet",
    );

    trim_conversation(&pool, worked.id).await.unwrap();
    // Put back with the archiving it was made under, so that the second life
    // below is a life the comparison can tell from the first.
    trimmed_days_ago(&pool, worked.id, 9).await;

    assert!(
        trimmed(&pool, worked.id).await.unwrap(),
        "and now something has",
    );

    // Back on the list, which stops the clock and leaves the mark: the page
    // still has a Capture with no chunks under it to account for.
    unarchive_conversation(&pool, worked.id).await.unwrap();

    assert!(
        trimmed(&pool, worked.id).await.unwrap(),
        "an unarchiving gives nothing back",
    );

    // And put away again, which makes it trimmable a second time — the one
    // state where the sweep's question and this one part company.
    archive_conversation(&pool, worked.id).await.unwrap();
    archived_days_ago(&pool, worked.id, 4).await;

    assert_eq!(
        trimmable(&pool, 3).await.unwrap(),
        [worked.id],
        "there is a trim to do on it again",
    );
    assert!(
        trimmed(&pool, worked.id).await.unwrap(),
        "and its first life is still missing what the first trim took",
    );
}

/// And a Conversation nobody has swept says so, whatever else is true of it.
#[tokio::test]
async fn a_conversation_no_cleanup_has_reached_is_not_trimmed() {
    let (_dir, pool) = fresh_pool().await;

    let worked = worked(&pool, "rate-limiting").await;

    assert!(!trimmed(&pool, worked.id).await.unwrap());
    assert!(
        !trimmed(&pool, 404).await.unwrap(),
        "and so does one that is not there at all",
    );
}

/// A delete leaves no row anywhere naming the Conversation, and the schema is
/// what says where to look.
///
/// Three assertions in one, because they are three halves of the same promise.
/// The walk covers every table SQLite says is a Conversation's — a table added
/// next year and not joined to it fails here rather than years later. The
/// fixture fills every one of them — a table joined to the walk but never
/// written in a test is a walk nobody has run. And after the delete they are
/// empty, this store having held one Conversation and nothing else.
#[tokio::test]
async fn a_delete_leaves_no_row_anywhere_that_names_the_conversation() {
    let (_dir, pool) = fresh_pool().await;
    let worked = owning(&pool, "rate-limiting").await;

    // What makes the rest of this a test of the order as well as of the
    // coverage: with the keys enforced, a walk that took a row something still
    // pointed at would fail rather than leave a mess nobody looked for.
    let (keys,): (i64,) = sqlx::query_as("PRAGMA foreign_keys")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(keys, 1, "this database enforces its foreign keys");

    let theirs = a_conversations_tables(&pool).await;
    let walked: BTreeSet<String> = deleted_tables().into_iter().map(str::to_owned).collect();

    let missed: Vec<&String> = theirs.difference(&walked).collect();

    assert!(
        missed.is_empty(),
        "these tables name a Conversation and the delete's walk does not reach \
         them: {missed:?}",
    );

    for table in &theirs {
        assert!(
            rows(&pool, table).await > 0,
            "the fixture leaves nothing in {table}, so the delete of it is \
             something this test never sees happen",
        );
    }

    assert_eq!(
        delete_conversation(&pool, worked.id).await.unwrap(),
        Deletion::Deleted
    );

    for table in &theirs {
        assert_eq!(
            rows(&pool, table).await,
            0,
            "{table} still holds a row of the Conversation that was deleted",
        );
    }

    assert!(
        load_conversation(&pool, worked.id).await.unwrap().is_none(),
        "and there is no Conversation of that id to load at all",
    );
}

/// A Conversation nobody archived is refused, for the trim's reason: the
/// archiving is what authorises the loss, and a delete is the whole of it.
#[tokio::test]
async fn what_was_never_archived_is_not_deleted() {
    let (_dir, pool) = fresh_pool().await;
    let worked = worked(&pool, "rate-limiting").await;

    unarchive_conversation(&pool, worked.id).await.unwrap();

    assert_eq!(
        delete_conversation(&pool, worked.id).await.unwrap(),
        Deletion::NotArchived
    );

    assert!(
        load_conversation(&pool, worked.id).await.unwrap().is_some(),
        "and it is where it was, a refusal being a refusal",
    );

    assert_eq!(
        delete_conversation(&pool, 404).await.unwrap(),
        Deletion::NoSuchConversation
    );
}

/// What is there to delete is what has been archived for longer than the days,
/// and a trim in its past says nothing about it either way.
///
/// The trim's list asks after the mark because a trim can be owed twice; this
/// one does not, because the row the mark lives in is one of the rows a delete
/// takes.
#[tokio::test]
async fn what_is_deletable_is_what_has_been_archived_for_long_enough() {
    let (_dir, pool) = fresh_pool().await;

    let old = worked(&pool, "rate-limiting").await;
    archived_days_ago(&pool, old.id, 31).await;

    let fresh = worked(&pool, "usage-limits").await;

    let back = worked(&pool, "window-rollover").await;
    archived_days_ago(&pool, back.id, 31).await;
    unarchive_conversation(&pool, back.id).await.unwrap();

    let cleaned = worked(&pool, "counter-reset").await;
    archived_days_ago(&pool, cleaned.id, 31).await;
    trim_conversation(&pool, cleaned.id).await.unwrap();

    assert_eq!(
        deletable(&pool, 30).await.unwrap(),
        [old.id, cleaned.id],
        "the two archived a month ago, one of them trimmed on the way past",
    );

    assert!(
        !deletable(&pool, 30).await.unwrap().contains(&fresh.id),
        "one archived a moment ago is not old enough to go",
    );
    assert!(
        !deletable(&pool, 30).await.unwrap().contains(&back.id),
        "and one the human has taken back out has no clock running at all",
    );

    delete_conversation(&pool, old.id).await.unwrap();
    delete_conversation(&pool, cleaned.id).await.unwrap();

    assert!(
        deletable(&pool, 30).await.unwrap().is_empty(),
        "and once they are gone there is nothing left to do",
    );
}

/// And the space a cleanup freed comes back to the filesystem, which is what
/// the whole feature is for.
///
/// Deleting rows is not reclaiming disk: SQLite marks the pages free inside the
/// file and leaves the file the size it was, so a human who turned the delete on
/// to get their disk back would get none of it. Asked of the free list and the
/// page count rather than of the file on disk, which is the same fact without
/// waiting on a checkpoint: pages nothing can reach before, and a smaller
/// database with none of them after.
#[tokio::test]
async fn a_cleanup_gives_the_space_back() {
    let (_dir, pool) = fresh_pool().await;
    let worked = worked(&pool, "rate-limiting").await;

    // Enough of it that the delete frees whole pages rather than parts of one:
    // what is being asked is whether the file is rewritten, and a database that
    // fit on one page either way could not answer.
    let event = start_capture(&pool, worked.id, Some("session"), None)
        .await
        .unwrap();

    for line in 0..200 {
        append_capture(
            &pool,
            event,
            &format!("{line}: the session said a great deal indeed\n"),
            &Summary {
                lines: line + 1,
                turns: Some(2),
                latest: "the session said a great deal indeed".to_owned(),
            },
        )
        .await
        .unwrap();
    }

    let before = pages(&pool).await;

    assert_eq!(
        delete_conversation(&pool, worked.id).await.unwrap(),
        Deletion::Deleted
    );

    assert!(
        free(&pool).await > 0,
        "a delete leaves pages nothing can reach, which is the thing to give back",
    );
    assert_eq!(
        pages(&pool).await,
        before,
        "and leaves the database exactly as big as it was",
    );

    reclaim(&pool).await.unwrap();

    assert_eq!(
        free(&pool).await,
        0,
        "and afterwards there are none of them"
    );
    assert!(
        pages(&pool).await < before,
        "and the database itself is smaller than it was",
    );
}

/// How many pages the database is, which is its size in the only unit SQLite
/// measures itself in.
async fn pages(pool: &SqlitePool) -> i64 {
    let (pages,): (i64,) = sqlx::query_as("PRAGMA page_count")
        .fetch_one(pool)
        .await
        .unwrap();

    pages
}

/// And how many of them are free: emptied by a delete, still inside the file,
/// and no use to anything outside it.
async fn free(pool: &SqlitePool) -> i64 {
    let (free,): (i64,) = sqlx::query_as("PRAGMA freelist_count")
        .fetch_one(pool)
        .await
        .unwrap();

    free
}
