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
//! All five are Processes written as data in built-in plugins of their own, and
//! the record holds each of them by the word it always held — so a
//! Conversation stored before any of that reads its Process and its state with
//! nothing rewritten.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    AdoptedPullRequest, Edited, Process, State, hold_pull_request, load_conversation,
    open_database, process, register_repo, save_brief, set_process, start_conversation, start_in,
};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

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

    let id = start_conversation(pool, repo, "rate-limiting", THIS_DEVICE, None)
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

    let id = start_conversation(pool, repo, "rate-limiting", THIS_DEVICE, None)
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

    assert_eq!(process(&pool, id).await.unwrap(), Process::develop());
    assert_eq!(
        loaded(&pool, id).await,
        Process::develop(),
        "and the loaded Conversation carries it, beside the Direction",
    );
}

/// And one holding a pull-request adoption reads Review, that being the Process
/// its path already was: Draft to Wrapping over somebody else's branch.
#[tokio::test]
async fn a_conversation_holding_a_pull_request_reads_review() {
    let (_dir, pool) = fresh_pool().await;
    let id = holding_a_pull_request(&pool).await;

    assert_eq!(process(&pool, id).await.unwrap(), Process::review());
    assert_eq!(loaded(&pool, id).await, Process::review());
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
            set_process(&pool, id, Process::tinker()).await.unwrap(),
            Edited::Saved,
            "picking on {how}",
        );
        assert_eq!(
            loaded(&pool, id).await,
            Process::tinker(),
            "picking on {how}"
        );

        // And a second pick is the human changing their mind rather than a
        // second Process: one row per Conversation, replaced.
        assert_eq!(
            set_process(&pool, id, Process::investigate())
                .await
                .unwrap(),
            Edited::Saved,
            "picking again on {how}",
        );
        assert_eq!(
            loaded(&pool, id).await,
            Process::investigate(),
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
        Process::develop(),
        Process::investigate(),
        Process::review(),
        Process::tinker(),
        Process::fix_merge_issues(),
    ] {
        assert_eq!(
            set_process(&pool, id, picked.clone()).await.unwrap(),
            Edited::Saved,
            "picking {picked:?}",
        );
        assert_eq!(process(&pool, id).await.unwrap(), picked);
    }
}

/// A plugin's Process round-trips by its plugin's uuid and its id, and picking
/// one of Verkstead's own after it leaves no plugin behind.
#[tokio::test]
async fn a_plugins_process_round_trips_by_its_plugin_and_its_id() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;
    let provided = Process {
        plugin: "5e0c9a7e-2f4b-4c1d-8a3e-6b9f0d2c7a41".to_owned(),
        id: "review".to_owned(),
    };

    assert_eq!(
        set_process(&pool, id, provided.clone()).await.unwrap(),
        Edited::Saved
    );
    assert_eq!(
        process(&pool, id).await.unwrap(),
        provided,
        "its id is `review` and it is still the plugin's, not Verkstead's Review"
    );

    set_process(&pool, id, Process::tinker()).await.unwrap();
    assert_eq!(process(&pool, id).await.unwrap(), Process::tinker());
}

/// The words themselves, which are the half of the round trip a database opened
/// by hand sees: lowercase and spelled out rather than a number nobody can look
/// up.
#[tokio::test]
async fn the_stored_words_are_the_ones_the_column_holds() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    for (picked, word) in [
        (Process::develop(), "develop"),
        (Process::investigate(), "investigate"),
        (Process::review(), "review"),
        (Process::tinker(), "tinker"),
        (Process::fix_merge_issues(), "fix-merge-issues"),
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

/// And a row written before a Process was the plugin providing it — the bare
/// word, with nothing in the column beside it — reads as the Process of the
/// bundled plugin that word belongs to. Which is the whole of how a database
/// from before this stage runs on: nothing is rewritten, and a row this build
/// writes for one of the five has that column empty too, so a device of the
/// cluster on an older release reads it as the word it always was.
#[tokio::test]
async fn a_bare_word_reads_as_the_plugin_that_word_belongs_to() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    for word in [
        "develop",
        "investigate",
        "review",
        "tinker",
        "fix-merge-issues",
    ] {
        sqlx::query(
            "INSERT INTO processes (conversation_id, process, plugin) VALUES (?, ?, NULL)
             ON CONFLICT (conversation_id)
                 DO UPDATE SET process = excluded.process, plugin = NULL",
        )
        .bind(id)
        .bind(word)
        .execute(&pool)
        .await
        .unwrap();

        let read = process(&pool, id).await.unwrap();
        assert_eq!(read.id, word);
        assert_eq!(
            read,
            Process::named(word).expect("one of the five"),
            "{word} is the Process of the plugin providing it",
        );
        assert!(read.built_in(), "{word} is one of the five");

        // And written back by this build, that column is still empty.
        set_process(&pool, id, read).await.unwrap();
        let (plugin,): (Option<String>,) =
            sqlx::query_as("SELECT plugin FROM processes WHERE conversation_id = ?")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(plugin, None, "nothing is rewritten for {word}");
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

    start_in(
        &pool,
        id,
        &State::named("grilling"),
        "c0ffee",
        Path::new("/state/worktrees/rate-limiting"),
        &[],
    )
    .await
    .unwrap();

    assert_eq!(
        set_process(&pool, id, Process::tinker()).await.unwrap(),
        Edited::NotDrafting,
    );
    assert_eq!(
        loaded(&pool, id).await,
        Process::develop(),
        "and nothing was written, so the reading still stands",
    );
}

#[tokio::test]
async fn there_is_no_conversation_to_pick_a_process_on() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        set_process(&pool, 404, Process::develop()).await.unwrap(),
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
        set_process(&pool, id, Process::fix_merge_issues())
            .await
            .unwrap();
        pool.close().await;
        id
    };

    let pool = open_database(&database).await.unwrap();

    assert_eq!(loaded(&pool, id).await, Process::fix_merge_issues());
}

/// A Conversation of each of the five Processes, stored by a Verkstead from
/// before there were plugin Processes — the `processes` table with no `plugin`
/// column, and each state the word it always held — reads its Process and its
/// state on a database opened again, with nothing rewritten.
///
/// Develop in every one of its five states, and its `implementing` under each
/// Direction: what the state column holds is the word the manifest now names
/// the state by, so the word that was stored is the word that is read.
#[tokio::test]
async fn the_five_stored_before_processes_were_data_read_on() {
    use verkstead_schema::Direction;
    use verkstead_store::State;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("verkstead.db");
    let stored = [
        (
            Process::develop(),
            "grilling",
            State::named("grilling"),
            None,
        ),
        (
            Process::develop(),
            "implementing",
            State::named("implementing"),
            Some(Direction::Inline),
        ),
        (
            Process::develop(),
            "implementing",
            State::named("implementing"),
            Some(Direction::TaskList),
        ),
        (
            Process::develop(),
            "implementing",
            State::named("implementing"),
            Some(Direction::Roadmap),
        ),
        (
            Process::develop(),
            "wrapping",
            State::named("wrapping"),
            None,
        ),
        (
            Process::develop(),
            "follow-up",
            State::named("follow-up"),
            None,
        ),
        (
            Process::develop(),
            "investigating",
            State::named("investigating"),
            None,
        ),
        (
            Process::tinker(),
            "follow-up",
            State::named("follow-up"),
            None,
        ),
        (
            Process::investigate(),
            "investigating",
            State::named("investigating"),
            None,
        ),
        (
            Process::review(),
            "wrapping",
            State::named("wrapping"),
            None,
        ),
        (
            Process::fix_merge_issues(),
            "wrapping",
            State::named("wrapping"),
            None,
        ),
    ];

    let mut ids = Vec::new();
    {
        let pool = open_database(&path).await.unwrap();
        for (picked, state, _, direction) in &stored {
            let id = drafting(&pool).await;
            set_process(&pool, id, picked.clone()).await.unwrap();
            // The row as a Verkstead from before would have left it: a
            // Direction is picked on a grilling, and this Conversation is
            // being stood up in the state it was found in.
            if let Some(direction) = direction {
                sqlx::query("INSERT INTO directions (conversation_id, direction) VALUES (?, ?)")
                    .bind(id)
                    .bind(match direction {
                        Direction::Inline => "inline",
                        Direction::TaskList => "task-list",
                        Direction::Roadmap => "roadmap",
                    })
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            sqlx::query("UPDATE conversations SET state = ? WHERE id = ?")
                .bind(state)
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
            ids.push(id);
        }

        // And the table as it was before it had anywhere to name a plugin.
        sqlx::query("ALTER TABLE processes DROP COLUMN plugin")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
    }

    let pool = open_database(&path).await.unwrap();
    for (id, (picked, _, lifecycle, direction)) in ids.into_iter().zip(stored) {
        let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
        assert_eq!(conversation.process, picked);
        assert_eq!(conversation.state, lifecycle, "{picked:?}");
        // The state's own id beside it, which is the word the column holds:
        // the five name their states with exactly these words, so a row
        // stored before any of this is worded off its Process's manifest with
        // nothing rewritten.
        assert_eq!(
            conversation.state.declared(),
            match lifecycle.verksteads() {
                true => None,
                false => Some(lifecycle.stored()),
            },
            "{picked:?}"
        );
        assert_eq!(conversation.direction, direction, "{picked:?}");
    }
}
