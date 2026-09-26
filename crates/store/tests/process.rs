//! What kind of work a Conversation is for: the Process on its record.
//!
//! Two halves, and the second is the interesting one. A Process picked on a
//! drafting Conversation is written to a side table and read straight back —
//! that much is the direction's shape exactly. What is new here is the reading
//! that answers where nothing was written: every Conversation has a Process,
//! including every one started before there were any, so a missing row is a
//! reading rather than a gap. Nothing is backfilled into an old row, and the
//! same rule answers for whichever kind of row a Conversation lacks.
//!
//! Only Develop can start anything yet. All five are here all the same, because
//! the record reads and writes every one of them and what a stage after this
//! adds is a start path rather than a variant.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    AdoptedPullRequest, Edited, Process, hold_pull_request, load_conversation, open_database,
    process, register_repo, save_brief, set_process, start_conversation, start_grilling,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// The Repo everything here is worked in, registered once and found again by
/// whichever fixture asks for it second.
async fn repo(pool: &SqlitePool) -> i64 {
    register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .map(|repo| repo.id)
        .unwrap_or(1)
}

/// An ordinary Draft: a Repo, a branch and a Brief, and nothing picked on it.
async fn drafting(pool: &SqlitePool) -> i64 {
    let repo = repo(pool).await;

    let id = start_conversation(pool, repo, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo is registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();

    id
}

/// And one holding a pull-request adoption, which is what the retired *Wrap up a
/// pull request* level left behind and what the Review start writes now.
async fn holding_a_pull_request(pool: &SqlitePool) -> i64 {
    let repo = repo(pool).await;

    let id = start_conversation(pool, repo, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    hold_pull_request(
        pool,
        id,
        &AdoptedPullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            head: "rate-limiting".to_owned(),
            base: "main".to_owned(),
        },
    )
    .await
    .unwrap();

    id
}

/// The Process the loaded Conversation carries, which is what every reader but
/// the write itself goes through.
async fn loaded(pool: &SqlitePool, id: i64) -> Process {
    load_conversation(pool, id)
        .await
        .unwrap()
        .expect("the Conversation is there")
        .process
}

/// A Conversation nobody has picked a Process on reads Develop, which is the
/// ladder there was before there were any.
#[tokio::test]
async fn a_conversation_with_no_row_of_its_own_reads_develop() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    assert_eq!(process(&pool, id).await.unwrap(), Process::Develop);
    assert_eq!(
        loaded(&pool, id).await,
        Process::Develop,
        "and the loaded Conversation carries it, beside the Direction",
    );
}

/// And one holding a pull-request adoption reads Review, that being the Process
/// its path already was: Draft to Wrapping over somebody else's branch.
#[tokio::test]
async fn a_conversation_holding_a_pull_request_reads_review() {
    let (_dir, pool) = fresh_pool().await;
    let id = holding_a_pull_request(&pool).await;

    assert_eq!(process(&pool, id).await.unwrap(), Process::Review);
    assert_eq!(loaded(&pool, id).await, Process::Review);
}

/// A Process written over either reading is what reads back: the row is the
/// answer wherever there is one, and the reading is only what stands where
/// there is none.
#[tokio::test]
async fn a_process_written_over_either_reading_is_what_reads_back() {
    let (_dir, pool) = fresh_pool().await;

    for (how, id) in [
        ("one that would have read Develop", drafting(&pool).await),
        (
            "one that would have read Review",
            holding_a_pull_request(&pool).await,
        ),
    ] {
        assert_eq!(
            set_process(&pool, id, Process::Tinker).await.unwrap(),
            Edited::Saved,
            "picking on {how}",
        );
        assert_eq!(loaded(&pool, id).await, Process::Tinker, "picking on {how}");

        // And a second pick is the human changing their mind rather than a
        // second Process: one row per Conversation, replaced.
        assert_eq!(
            set_process(&pool, id, Process::Investigate).await.unwrap(),
            Edited::Saved,
            "picking again on {how}",
        );
        assert_eq!(
            loaded(&pool, id).await,
            Process::Investigate,
            "the latest pick is the one that stands, on {how}",
        );
    }
}

/// All five round-trip through their stored words. Only Develop can start
/// anything yet, and the record holds every one of them all the same — what a
/// later stage adds is a start path, never a variant.
#[tokio::test]
async fn all_five_round_trip_through_their_stored_words() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    for picked in [
        Process::Develop,
        Process::Investigate,
        Process::Review,
        Process::Tinker,
        Process::FixMergeIssues,
    ] {
        assert_eq!(
            set_process(&pool, id, picked).await.unwrap(),
            Edited::Saved,
            "picking {picked:?}",
        );
        assert_eq!(process(&pool, id).await.unwrap(), picked);
    }
}

/// The words themselves, which are the half of the round trip a database opened
/// by hand sees: lowercase and spelled out rather than a number nobody can look
/// up.
#[tokio::test]
async fn the_stored_words_are_the_ones_the_column_holds() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    for (picked, word) in [
        (Process::Develop, "develop"),
        (Process::Investigate, "investigate"),
        (Process::Review, "review"),
        (Process::Tinker, "tinker"),
        (Process::FixMergeIssues, "fix-merge-issues"),
    ] {
        set_process(&pool, id, picked).await.unwrap();

        let (stored,): (String,) =
            sqlx::query_as("SELECT process FROM processes WHERE conversation_id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(stored, word);
    }
}

/// A word this Verkstead has never heard of is a database written by one it
/// does not understand, which is worth saying rather than guessing past — and
/// the word is in the message, the way an unknown state's is.
#[tokio::test]
async fn an_unknown_word_fails_with_the_word_in_the_message() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    sqlx::query("INSERT INTO processes (conversation_id, process) VALUES (?, 'bisecting')")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let refused = process(&pool, id).await.unwrap_err().to_string();

    assert!(
        refused.contains("bisecting"),
        "the word that could not be read is in the message: {refused}",
    );

    assert!(
        load_conversation(&pool, id).await.is_err(),
        "and the load refuses rather than handing back a guess, as it does for \
         a state it cannot parse",
    );
}

/// A pick is a Draft's freedom, and the same two questions the branch name and
/// the base commit are refused off refuse this: where the Conversation has got
/// to, and whether its branch has been cut.
///
/// Which is the whole of how a Process is frozen at Start. Nothing is written
/// when the work begins — the refusal alone does it — so a Develop Conversation
/// nobody touched the picker on keeps no row at all.
#[tokio::test]
async fn a_pick_is_refused_from_the_moment_the_branch_is_cut() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    start_grilling(
        &pool,
        id,
        "c0ffee",
        Path::new("/state/worktrees/rate-limiting"),
        &[],
    )
    .await
    .unwrap();

    assert_eq!(
        set_process(&pool, id, Process::Tinker).await.unwrap(),
        Edited::NotDrafting,
    );
    assert_eq!(
        loaded(&pool, id).await,
        Process::Develop,
        "and nothing was written, so the reading still stands",
    );
}

#[tokio::test]
async fn there_is_no_conversation_to_pick_a_process_on() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        set_process(&pool, 404, Process::Develop).await.unwrap(),
        Edited::NoSuchConversation,
    );
}

/// A pick survives a restart, the row being the only thing left saying the
/// human ever touched the picker.
#[tokio::test]
async fn a_process_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&database).await.unwrap();
        let id = drafting(&pool).await;
        set_process(&pool, id, Process::FixMergeIssues)
            .await
            .unwrap();
        pool.close().await;
        id
    };

    let pool = open_database(&database).await.unwrap();

    assert_eq!(loaded(&pool, id).await, Process::FixMergeIssues);
}
