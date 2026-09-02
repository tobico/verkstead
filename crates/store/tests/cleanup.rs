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
//! than out of a list written here — see [`a_conversations_tables`], which is
//! what makes this a test a table added next year has to answer to.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_schema::{QuestionSet, Response};
use verkstead_store::{
    Account, Adding, Ask, Commit, CompanionWorktree, Decision, Deletion, Merging, Pairing,
    ProfileFacts, PullRequest, Rollup, Settlements, Standing, Summary, Trimming, WaitingOn,
    add_companion, append_capture, append_transcript, archive_conversation, ask, capture,
    close_conversation, create_profile, deletable, delete_conversation, deleted_tables,
    load_conversation, load_response, lock_set, nothing_else, open_database, pick_direction,
    place_conversations, reclaim, record_addressed_comments, record_backlog, record_check_rollup,
    record_commit, record_conflict_fix_attempt, record_fix_attempt, record_merging,
    record_pull_request, record_share, record_share_comment, record_standing, register_repo,
    save_brief, session_id, set_grilling_pairing, settle_wrap_up, skip_review, stamp_unseen,
    start_capture, start_conversation, start_grilling, start_implementing, stop, submit_response,
    timeline, transcript, trim_conversation, trimmable, trimmed, unarchive_conversation,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// The Repo everything here is worked in, registered once and found afterwards.
async fn repo(pool: &SqlitePool) -> i64 {
    register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .map(|repo| repo.id)
        .unwrap_or(1)
}

/// A Conversation with one of everything a trim has an opinion about, and the
/// handles the assertions read it back by.
struct Worked {
    id: i64,

    /// The Event one session printed into: the Capture, the Transcript and the
    /// name that session ran under all hang off this.
    event: i64,

    /// The Set asked on the way, so that what a Response survives can be read
    /// back by itself as well as off the Timeline.
    set: i64,
}

/// Take one the whole way: a session that printed and kept a log, a Set asked
/// and answered, a commit, a pull request — and then closed and put away, which
/// is the only state a trim will look at.
async fn worked(pool: &SqlitePool, branch: &str) -> Worked {
    let repo = repo(pool).await;

    let id = start_conversation(pool, repo, branch)
        .await
        .unwrap()
        .expect("the Repo is registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();

    let event = printed(pool, id, branch, "the session said a great deal").await;

    let set = ask(pool, id, &asked(), Ask::Blocking)
        .await
        .unwrap()
        .expect("the Conversation is there to ask from")
        .id;

    submit_response(pool, &Settlements::new(4), set, &Response::default())
        .await
        .unwrap();

    start_grilling(
        pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        &PathBuf::from("/state/worktrees").join(branch),
        &[],
    )
    .await
    .unwrap();

    pick_direction(pool, id, verkstead_schema::Direction::TaskList)
        .await
        .unwrap();
    record_backlog(pool, id).await.unwrap();
    start_implementing(pool, id).await.unwrap();

    record_commit(
        pool,
        id,
        repo,
        &Commit {
            sha: "d41f8a3b6c2e91750f4a8c3d5b7e2f10a9c6d4b8".to_owned(),
            subject: "feat: count the requests".to_owned(),
            files: 7,
            insertions: 412,
            deletions: 3,
            summary: Some("The counter moves out of the process.".to_owned()),
            repo: None,
            merge: false,
        },
    )
    .await
    .unwrap()
    .unwrap();

    record_pull_request(
        pool,
        id,
        repo,
        &PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    close_conversation(pool, id).await.unwrap();
    archive_conversation(pool, id).await.unwrap();

    Worked { id, event, set }
}

/// One session's worth of bulk: the Capture, the log it kept of itself, and the
/// name Verkstead ran it under — with the summary the Timeline card is drawn
/// from beside them.
async fn printed(pool: &SqlitePool, id: i64, session: &str, said: &str) -> i64 {
    let profile = create_profile(
        pool,
        &ProfileFacts {
            name: session.to_owned(),
            account: Account::Codex {
                home: PathBuf::from("/watched/accounts/work/.codex"),
            },
            models: vec!["gpt-5".to_owned()],
        },
    )
    .await
    .unwrap()
    .expect("nothing is called that yet");

    let pairing = Pairing {
        profile,
        model: Some("gpt-5".to_owned()),
    };

    let event = start_capture(pool, id, Some(session), Some(&pairing))
        .await
        .unwrap();

    append_capture(
        pool,
        event,
        &format!("{said}\n"),
        &Summary {
            lines: 1,
            turns: Some(2),
            latest: said.to_owned(),
        },
    )
    .await
    .unwrap();

    append_transcript(
        pool,
        event,
        &[format!(r#"{{"type":"assistant","text":"{said}"}}"#)],
    )
    .await
    .unwrap();

    event
}

/// The smallest Set there is: one that asks nothing, which [`Response::default`]
/// answers.
fn asked() -> QuestionSet {
    serde_saphyr::from_str(
        "title: Where should the counter live?\nproject: verkstead\nquestions: []\n",
    )
    .unwrap()
}

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

/// The whole of what the store holds about one Conversation, for the delete to
/// be held against: a row in every table the schema says is a Conversation's.
///
/// Deliberately more than [`worked`], and deliberately not shared with it — the
/// trim's tests are about a boundary between two kinds of row, and this is about
/// there being nothing left anywhere. What it is filling is checked rather than
/// trusted: the test below asserts that every table the schema names has a row
/// in it before the delete, so a fixture that stopped filling one is a failing
/// test rather than a delete nobody is checking.
async fn owning(pool: &SqlitePool, branch: &str) -> Worked {
    let repo = repo(pool).await;

    let companion = register_repo(pool, Path::new("/watched/askance"), "askance", "main")
        .await
        .unwrap()
        .map(|repo| repo.id)
        .unwrap_or(2);

    let id = start_conversation(pool, repo, branch)
        .await
        .unwrap()
        .expect("the Repo is registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();

    // While it is still a draft, which is the only time these are settled: the
    // other repository it is worked in, the model one role runs on, and the role
    // that runs no session at all.
    assert_eq!(
        add_companion(pool, id, companion).await.unwrap(),
        Adding::Added
    );

    let profile = create_profile(
        pool,
        &ProfileFacts {
            name: format!("{branch}-grilling"),
            account: Account::Codex {
                home: PathBuf::from("/watched/accounts/work/.codex"),
            },
            models: vec!["gpt-5".to_owned()],
        },
    )
    .await
    .unwrap()
    .expect("nothing is called that yet");

    set_grilling_pairing(pool, id, profile.id, Some("gpt-5"))
        .await
        .unwrap();
    skip_review(pool, id).await.unwrap();

    let event = printed(pool, id, branch, "the session said a great deal").await;

    // Three Sets, because a Set can end three ways and each way is its own row:
    // answered, stored for nobody, and locked unanswered.
    let set = ask(pool, id, &asked(), Ask::Blocking)
        .await
        .unwrap()
        .expect("the Conversation is there to ask from")
        .id;

    // Answered with Nothing else, which is what writes the mark saying the round
    // is over.
    submit_response(
        pool,
        &Settlements::new(4),
        set,
        &Response {
            nothing_else: true,
            ..Response::default()
        },
    )
    .await
    .unwrap();

    assert!(
        nothing_else(pool, id).await.unwrap(),
        "the round is marked as over",
    );

    ask(pool, id, &asked(), Ask::Deferred).await.unwrap();

    let unanswered = ask(pool, id, &asked(), Ask::Blocking)
        .await
        .unwrap()
        .expect("the Conversation is there to ask from")
        .id;

    lock_set(pool, &Settlements::new(4), unanswered)
        .await
        .unwrap();

    start_grilling(
        pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        &PathBuf::from("/state/worktrees").join(branch),
        &[CompanionWorktree {
            repo_id: companion,
            path: PathBuf::from("/state/worktrees").join(format!("{branch}-askance")),
            base_commit: Some("0b7c2e91f4a8d3c5b6e7f10a9c6d4b82d41f8a3b".to_owned()),
        }],
    )
    .await
    .unwrap();

    pick_direction(pool, id, verkstead_schema::Direction::TaskList)
        .await
        .unwrap();
    record_backlog(pool, id).await.unwrap();
    start_implementing(pool, id).await.unwrap();

    record_commit(
        pool,
        id,
        repo,
        &Commit {
            sha: "d41f8a3b6c2e91750f4a8c3d5b7e2f10a9c6d4b8".to_owned(),
            subject: "feat: count the requests".to_owned(),
            files: 7,
            insertions: 412,
            deletions: 3,
            summary: Some("The counter moves out of the process.".to_owned()),
            repo: None,
            merge: false,
        },
    )
    .await
    .unwrap()
    .unwrap();

    record_pull_request(
        pool,
        id,
        repo,
        &PullRequest {
            number: 41,
            title: "Rate limiting".to_owned(),
            url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
            repo: None,
        },
    )
    .await
    .unwrap();

    // What GitHub last said about it, and how far the wrap-up got.
    record_check_rollup(pool, id, Rollup::Passed).await.unwrap();
    record_merging(pool, id, repo, Merging::Cleanly)
        .await
        .unwrap();
    record_standing(pool, id, repo, Standing::Open)
        .await
        .unwrap();
    settle_wrap_up(pool, id, WaitingOn::Review).await.unwrap();
    record_fix_attempt(pool, id, repo, "build").await.unwrap();
    record_conflict_fix_attempt(pool, id, repo).await.unwrap();
    record_addressed_comments(pool, id, repo, &["IC_kwDO".to_owned()])
        .await
        .unwrap();

    // What was shared of it, where it sits, and that nobody has read it.
    record_share(pool, id, "https://share.example/rate-limiting")
        .await
        .unwrap();
    record_share_comment(pool, id).await.unwrap();
    place_conversations(pool, &[id]).await.unwrap();
    stamp_unseen(pool, id).await.unwrap();

    // And the stop, whose Notice is the one row pointing back the other way: the
    // Conversation names an Event of its own, so a delete that took the Events
    // first would be one SQLite refused.
    stop(
        pool,
        id,
        Decision::Verkstead,
        "the account is out of window\n",
        None,
    )
    .await
    .unwrap();

    close_conversation(pool, id).await.unwrap();
    archive_conversation(pool, id).await.unwrap();

    // Trimmed, and then lived in again: the mark is a row a delete has to take
    // as well, and the bulk it took has to be back for the assertions to mean
    // anything.
    trim_conversation(pool, id).await.unwrap();
    unarchive_conversation(pool, id).await.unwrap();
    printed(pool, id, "second-session", "and said a great deal more").await;
    archive_conversation(pool, id).await.unwrap();

    written_straight_in(pool, id, companion, event).await;

    Worked { id, event, set }
}

/// The six rows no press could leave where this fixture ends, written straight
/// in.
///
/// Two are a Verkstead of before — an open Pause is how an account out of window
/// was recorded before there were stops. Two belong to starts this Conversation
/// did not have: a stage's branch, and the roadmap an adoption is of. And two
/// are the worktrees, which closing sweeps away, so an archived Conversation
/// never really has them.
///
/// Which is the point of writing them in rather than leaving them out. The walk
/// takes every row naming a Conversation, and *this cannot happen* is not
/// something it is entitled to assume about a table it has to empty: a row that
/// got there somehow is a row the delete has to survive.
async fn written_straight_in(pool: &SqlitePool, id: i64, companion: i64, event: i64) {
    sqlx::query(
        "INSERT INTO pauses (event_id, conversation_id, profile, said, resets_at)
         VALUES (?, ?, 'work', 'the account is out of window', NULL)",
    )
    .bind(event)
    .bind(id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO stage_branches (conversation_id, stacks_on) VALUES (?, NULL)")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO adoptions (conversation_id, roadmap) VALUES (?, 'missing-roles')")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO wrap_up_narrowings (conversation_id, at)
         VALUES (?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO worktrees (conversation_id, path) VALUES (?, '/state/worktrees/x')")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO companion_worktrees (conversation_id, repo_id, path, base_commit)
         VALUES (?, ?, '/state/worktrees/x-askance', NULL)",
    )
    .bind(id)
    .bind(companion)
    .execute(pool)
    .await
    .unwrap();
}

/// Every table this database says holds rows belonging to a Conversation, read
/// out of the schema rather than written down.
///
/// The walk down is the foreign keys: a table naming `conversations` is a
/// Conversation's, a table naming one of those is one too, and so on until
/// nothing more joins. Which is why `repos` and `profiles` are not caught by it
/// — a Conversation names *them*, not the other way about, and shared things are
/// exactly the things it points at.
///
/// One link is seeded rather than followed, and it is the only one the schema
/// cannot say the direction of: a Question Set is asked from a Conversation, and
/// what says so is `set_events`, which names both. So `question_sets` is put in
/// at the start and everything hanging off it — the Response, the lock, the
/// deferral, the ending — is found from there.
async fn a_conversations_tables(pool: &SqlitePool) -> BTreeSet<String> {
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    let mut points_at: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for (table,) in &tables {
        let named: Vec<(String,)> = sqlx::query_as(&format!(
            "SELECT \"table\" FROM pragma_foreign_key_list('{table}')"
        ))
        .fetch_all(pool)
        .await
        .unwrap();

        points_at.insert(table.clone(), named.into_iter().map(|(at,)| at).collect());
    }

    let mut owned = BTreeSet::from(["conversations".to_owned(), "question_sets".to_owned()]);

    loop {
        let joined: Vec<String> = points_at
            .iter()
            .filter(|(table, at)| {
                !owned.contains(*table) && at.iter().any(|named| owned.contains(named))
            })
            .map(|(table, _)| table.clone())
            .collect();

        if joined.is_empty() {
            return owned;
        }

        owned.extend(joined);
    }
}

/// How many rows one table holds, the store being one Conversation's here.
async fn rows(pool: &SqlitePool, table: &str) -> i64 {
    let (rows,): (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap();

    rows
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
