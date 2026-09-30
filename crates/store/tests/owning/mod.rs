#![allow(dead_code)]

//! A Conversation with one of everything, for the tests that have to be held
//! against the whole of what the store keeps about one.
//!
//! Two of them want it, and for the same reason from opposite ends. The
//! **delete** has to leave no row anywhere, and a **transfer's slice** has to
//! carry every row there is: both are promises about a set of tables nobody
//! wrote down, and both are kept honest by the same thing — [`owning`] fills
//! every table the schema says is a Conversation's, and
//! [`a_conversations_tables`] is what says which those are.
//!
//! So the fixture lives here rather than in either suite. A table added next
//! year is one test failing in both places until somebody has decided what
//! happens to it, which is the whole point of reading the tables off the schema
//! rather than off a list.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::SqlitePool;
use verkstead_schema::{QuestionSet, Response};
use verkstead_store::{
    Account, Adding, Ask, Commit, CompanionMode, CompanionWorktree, Decision, Edited, Lifecycle,
    Merging, Origin, Pairing, PendingAddition, PendingForm, PendingUpgrade, Process, ProfileFacts,
    PullRequest, Rollup, Settlements, Standing, Summary, WaitingOn, add_companion, append_capture,
    append_transcript, archive_conversation, ask, ask_to_transfer, attach, attach_mcp_server,
    close_conversation, create_profile, end_session, lock_set, nothing_else, open_database,
    open_pending_steer, pick_direction, record_addressed_comments, record_backlog,
    record_check_rollup, record_commit, record_conflict_fix_attempt, record_delivery,
    record_fix_attempt, record_merging, record_pull_request, record_share, record_share_comment,
    record_standing, register_repo, save_brief, save_pending_steer, set_grilling_pairing,
    set_process, set_target, settle_wrap_up, skip_review, stamp_unseen, start_capture,
    start_conversation, start_grilling, start_implementing, stop, submit_response,
    trim_conversation, unarchive_conversation,
};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// A pool over a fresh database, plus the directory keeping it alive.
pub async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// The Repo everything here is worked in, registered once and found afterwards.
pub async fn repo(pool: &SqlitePool) -> i64 {
    register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .map(|repo| repo.id)
        .unwrap_or(1)
}

/// A Conversation with one of everything a trim has an opinion about, and the
/// handles the assertions read it back by.
pub struct Worked {
    pub id: i64,

    /// The Event one session printed into: the Capture, the Transcript and the
    /// name that session ran under all hang off this.
    pub event: i64,

    /// The Set asked on the way, so that what a Response survives can be read
    /// back by itself as well as off the Timeline.
    pub set: i64,
}

/// Take one the whole way: a session that printed and kept a log, a Set asked
/// and answered, a commit, a pull request — and then closed and put away, which
/// is the only state a trim will look at.
pub async fn worked(pool: &SqlitePool, branch: &str) -> Worked {
    let repo = repo(pool).await;

    let id = start_conversation(pool, repo, branch, THIS_DEVICE)
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
            head: Some("rate-limiting".to_owned()),
            base: None,
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
/// from beside them, and how the session ended.
pub async fn printed(pool: &SqlitePool, id: i64, session: &str, said: &str) -> i64 {
    let profile = create_profile(
        pool,
        &ProfileFacts {
            name: Some(session.to_owned()),
            account: Account::Codex {
                home: PathBuf::from("/watched/accounts/work/.codex"),
            },
            models: vec!["gpt-5".to_owned()],
            memory: true,
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

    end_session(pool, event, Some(1), Duration::from_millis(400), true)
        .await
        .unwrap();

    event
}

/// The smallest Set there is: one that asks nothing, which [`Response::default`]
/// answers.
pub fn asked() -> QuestionSet {
    serde_saphyr::from_str(
        "title: Where should the counter live?\nproject: verkstead\nquestions: []\n",
    )
    .unwrap()
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
pub async fn owning(pool: &SqlitePool, branch: &str) -> Worked {
    let repo = repo(pool).await;

    let companion = register_repo(pool, Path::new("/watched/askance"), "askance", "main")
        .await
        .unwrap()
        .map(|repo| repo.id)
        .unwrap_or(2);

    let id = start_conversation(pool, repo, branch, THIS_DEVICE)
        .await
        .unwrap()
        .expect("the Repo is registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();

    // While it is still a draft, which is the only time these are settled: the
    // other repository it is worked in, the Process it runs, what it is pointed
    // at, the model one role runs on, and the role that runs no session at all.
    assert_eq!(
        add_companion(pool, id, companion).await.unwrap(),
        Adding::Added
    );

    assert_eq!(
        set_process(pool, id, Process::Develop).await.unwrap(),
        Edited::Saved,
    );

    assert_eq!(
        set_target(pool, id, Some("#41")).await.unwrap(),
        Edited::Saved,
    );

    let profile = create_profile(
        pool,
        &ProfileFacts {
            name: Some(format!("{branch}-grilling")),
            account: Account::Codex {
                home: PathBuf::from("/watched/accounts/work/.codex"),
            },
            models: vec!["gpt-5".to_owned()],
            memory: true,
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
        nothing_else(pool, id, Lifecycle::FollowUp).await.unwrap(),
        "the round is marked as over",
    );

    // And handed over, which is the row saying its Answers reached a session.
    record_delivery(pool, set).await.unwrap();

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
            head: Some("rate-limiting".to_owned()),
            base: None,
            repo: None,
        },
    )
    .await
    .unwrap();

    // What GitHub last said about it, and how far the wrap-up got.
    record_check_rollup(pool, id, repo, 41, Rollup::Passed)
        .await
        .unwrap();
    record_merging(pool, id, repo, 41, Merging::Cleanly)
        .await
        .unwrap();
    record_standing(pool, id, repo, 41, Standing::Open)
        .await
        .unwrap();
    settle_wrap_up(pool, id, WaitingOn::Review).await.unwrap();
    record_fix_attempt(pool, id, repo, 41, "build")
        .await
        .unwrap();
    record_conflict_fix_attempt(pool, id, repo).await.unwrap();
    record_addressed_comments(pool, id, repo, &["IC_kwDO".to_owned()])
        .await
        .unwrap();

    // What was shared of it, where it sits, and that nobody has read it.
    record_share(pool, id, "https://share.example/rate-limiting")
        .await
        .unwrap();
    record_share_comment(pool, id).await.unwrap();

    // And a file the human put on it, which is the one sidecar there may be
    // several of.
    attach(pool, id, Origin::Brief, "burst.csv", 48_112)
        .await
        .unwrap();

    // One of each origin, because the second names a Set and the first names
    // none: a row pointing at `question_sets` is one a delete has to take before
    // the Set it points at, and a fixture carrying only the Brief's would not
    // notice a delete that took them the other way round. The label is the
    // server's to check against what the Set asks — the record takes the row as
    // it stands.
    attach(
        pool,
        id,
        Origin::Answer {
            set,
            label: "Q1".to_owned(),
        },
        "the-counter-we-have.rs",
        2_184,
    )
    .await
    .unwrap();

    // And an MCP server it was given, which is the other thing attached at that
    // control: a name rather than a file, in a table of its own.
    attach_mcp_server(pool, id, "docs").await.unwrap();

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

    // And a steer somebody started and left, which is a row beside the
    // Conversation rather than on its Timeline — with a companion row of each
    // kind on it, those being tables of their own.
    //
    // After the close rather than before it, because a close takes a pending
    // steer away: the form is about where the work goes next, and closing is
    // the end of the work. What this fixture is for is a row in every table,
    // so the press that leaves one comes last.
    open_pending_steer(pool, id).await.unwrap();
    save_pending_steer(
        pool,
        id,
        &PendingForm {
            target: Some(Lifecycle::Implementing),
            instruction: Some("take the modal out".to_owned()),
            added: vec![PendingAddition {
                repo_id: repo,
                mode: CompanionMode::ReadOnly,
                base_ref: None,
                branch: String::new(),
            }],
            upgraded: vec![PendingUpgrade {
                repo_id: companion,
                branch: String::new(),
            }],
            ..PendingForm::default()
        },
    )
    .await
    .unwrap();

    written_straight_in(pool, id, companion, event).await;

    Worked { id, event, set }
}

/// The rows no press could leave where this fixture ends, written straight in.
///
/// Two are a Verkstead of before — an open Pause is how an account out of window
/// was recorded before there were stops. Two belong to starts this Conversation
/// did not have: a stage's branch, and the roadmap an adoption is of. Two are the
/// worktrees, which closing sweeps away, so an archived Conversation never really
/// has them. And one is the mark saying a copy of this Conversation was
/// transferred to another device, which is not something offered on a Closed one.
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

    // And what a steer settled, hung off the same Event: a Conversation nobody
    // steered has none of these, and the walk still has to empty them for one
    // that was.
    sqlx::query(
        "INSERT INTO steers (event_id, conversation_id, digest, interrupt, profile_id, model)
         VALUES (?, ?, 0, 0, NULL, NULL)",
    )
    .bind(event)
    .bind(id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO steer_additions (event_id, repo_id, mode, base_ref, branch)
         VALUES (?, ?, 'read-only', NULL, '')",
    )
    .bind(event)
    .bind(companion)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO steer_upgrades (event_id, repo_id, branch) VALUES (?, ?, '')")
        .bind(event)
        .bind(companion)
        .execute(pool)
        .await
        .unwrap();

    // And where that steer came from, which is another row hung off the same
    // Event.
    sqlx::query("INSERT INTO steer_sources (event_id, state) VALUES (?, 'grilling')")
        .bind(event)
        .execute(pool)
        .await
        .unwrap();

    // And what its checkout was already holding uncommitted: one row per path,
    // and the NULL row a checkout that held nothing is recorded as.
    sqlx::query("INSERT INTO steer_scratch (event_id, repo_id, path) VALUES (?, ?, 'README.md')")
        .bind(event)
        .bind(companion)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO stage_branches (conversation_id, stacks_on) VALUES (?, NULL)")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO stage_roadmaps (conversation_id, roadmap) VALUES (?, 'missing-roles')",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();

    // And its place in the queue to join its roadmap's chain, taken when its
    // tasks finished.
    sqlx::query("INSERT INTO stage_joinings (conversation_id) VALUES (?)")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();

    sqlx::query("INSERT INTO adoptions (conversation_id, roadmap) VALUES (?, 'missing-roles')")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();

    // The other thing a Draft adopts. Never on one Conversation alongside the
    // roadmap above — a Draft adopts one thing or none — but this fixture is
    // filling every table a Conversation is named from rather than composing a
    // Conversation anybody could have made.
    sqlx::query(
        "INSERT INTO pull_request_adoptions (conversation_id, number, title, url, head, base)
         VALUES (?, 41, 'Rate limiting', 'https://github.com/tobico/verkstead/pull/41',
                 'rate-limiting', 'main')",
    )
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

    // And the mark saying a copy of this Conversation was handed to another
    // device, which nothing could have left on one that went on to be closed and
    // archived here — a transfer is not offered from Closed, and the copy left
    // behind is not worked again. The walk has to empty it all the same.
    sqlx::query("INSERT INTO transferred (conversation_id, device, live_as) VALUES (?, ?, 3)")
        .bind(id)
        .bind("0011223344556677889900aabbccddee")
        .execute(pool)
        .await
        .unwrap();

    // And a move somebody pressed for and nothing ever made, which is the same
    // kind of row one press earlier: a request outlives nothing here, because
    // what acts on one takes it away — and the walk has to reach it all the
    // same, a Conversation being deletable long after anybody pressed anything.
    ask_to_transfer(pool, id, "0011223344556677889900aabbccddee")
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
pub async fn a_conversations_tables(pool: &SqlitePool) -> BTreeSet<String> {
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
pub async fn rows(pool: &SqlitePool, table: &str) -> i64 {
    let (rows,): (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap();

    rows
}
