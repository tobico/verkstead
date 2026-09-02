//! What a Cleanup takes out of an archived Conversation, and what it leaves
//! behind.
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

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_schema::{QuestionSet, Response};
use verkstead_store::{
    Account, Ask, Commit, Pairing, ProfileFacts, PullRequest, Settlements, Summary, Trimming,
    append_capture, append_transcript, archive_conversation, ask, capture, close_conversation,
    create_profile, load_response, open_database, pick_direction, record_backlog, record_commit,
    record_pull_request, register_repo, save_brief, session_id, start_capture, start_conversation,
    start_grilling, start_implementing, submit_response, timeline, transcript, trim_conversation,
    trimmable, trimmed, unarchive_conversation,
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
