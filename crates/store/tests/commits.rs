//! Commits on a Conversation's Timeline: recorded once each, in the order they
//! were offered, and read back as the line the Timeline draws.
//!
//! What is being asked of the store here is *exactly once*. Whatever watches a
//! branch sweeps the whole of it every time it looks — a branch is not a queue
//! — so nearly every commit it offers has been recorded already, and the store
//! is what makes offering one twice cost nothing.
//!
//! And *once per repository*, which is the other half of the same rule: a
//! Conversation is swept in its own repo and in each read-write companion, and
//! two repositories are two histories.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Commit, Event, add_companion, commit, commit_repo, commits_landed, forget_commit,
    open_database, record_commit, recorded_commits, register_repo, start_conversation, timeline,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation to land commits on, and the Repo it is in — which is what a
/// commit is recorded against.
async fn conversation(pool: &SqlitePool) -> (i64, i64) {
    let repo = registered(pool, "verkstead").await;

    let id = start_conversation(pool, repo, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    (id, repo)
}

/// One more registered Repo, for the Conversations and the companions that want
/// a second one.
async fn registered(pool: &SqlitePool, name: &str) -> i64 {
    register_repo(pool, &Path::new("/watched").join(name), name, "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id
}

/// A commit as git would have described it, saying nothing about itself — which
/// is what a bookkeeping commit is, and what every commit recorded before
/// summaries were kept looks like.
///
/// Unlabeled, which is what the sweep offers and what the Conversation's own
/// repository reads back as. And an ordinary commit rather than a merge, which
/// is what all but one commit on any branch is.
fn landed(sha: &str, subject: &str) -> Commit {
    Commit {
        sha: sha.to_owned(),
        subject: subject.to_owned(),
        files: 2,
        insertions: 31,
        deletions: 4,
        summary: None,
        repo: None,
        merge: false,
    }
}

/// The same, with the account the agent wrote under its subject.
fn summarised(sha: &str, subject: &str, summary: &str) -> Commit {
    Commit {
        summary: Some(summary.to_owned()),
        ..landed(sha, subject)
    }
}

/// And the same as a merge: what a resolution session leaves behind where it
/// brought the base branch in and settled the conflicts.
fn merged(sha: &str, subject: &str) -> Commit {
    Commit {
        merge: true,
        ..landed(sha, subject)
    }
}

/// The commits on a Conversation's Timeline, in Timeline order.
async fn on_the_timeline(pool: &SqlitePool, id: i64) -> Vec<Commit> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.event {
            Event::Commit(commit) => Some(commit),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_commit_lands_on_the_timeline_as_what_it_changed() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    let event = record_commit(&pool, id, repo, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap()
        .expect("a commit on a Conversation that is there is recorded");

    assert_eq!(
        on_the_timeline(&pool, id).await,
        vec![landed("a1b2c3d", "feat: rate limiting")],
        "the Timeline says what the commit was called and how much it moved",
    );

    assert_eq!(
        commit(&pool, id, event).await.unwrap(),
        Some(landed("a1b2c3d", "feat: rate limiting")),
        "and the Event is what the details pane fetches the commit by",
    );
}

/// What the agent wrote under the subject is kept beside the commit, and comes
/// back with it — both to the details pane, which renders it above the diff, and
/// to the Timeline the pane was opened from.
#[tokio::test]
async fn a_commit_carries_the_summary_it_was_recorded_with() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    let written = "```mermaid\nflowchart LR\n  in --> out\n```\n\nA bucket per account.";

    let event = record_commit(
        &pool,
        id,
        repo,
        &summarised("a1b2c3d", "feat: rate limiting", written),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(
        commit(&pool, id, event).await.unwrap().unwrap().summary,
        Some(written.to_owned()),
        "the pane fetches the summary by the Event, unrendered",
    );

    assert_eq!(
        on_the_timeline(&pool, id).await,
        vec![summarised("a1b2c3d", "feat: rate limiting", written)],
        "and the Timeline has it too, beside the line it draws",
    );
}

/// A commit that said nothing about itself, which is the ordinary one: the row
/// beside it is simply absent, and that is also what every commit recorded
/// before summaries were kept looks like.
#[tokio::test]
async fn a_commit_with_no_summary_has_none() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    let event = record_commit(&pool, id, repo, &landed("a1b2c3d", "chore: plan the tasks"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        commit(&pool, id, event).await.unwrap().unwrap().summary,
        None
    );
    assert_eq!(on_the_timeline(&pool, id).await[0].summary, None);
}

/// Whether a commit is a merge is read off git once, when the sweep describes
/// it, and kept beside the commit: the Timeline the card is drawn on and the
/// pane it opens both read it back rather than asking git again.
#[tokio::test]
async fn a_merge_comes_back_a_merge() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    let event = record_commit(&pool, id, repo, &merged("a1b2c3d", "Merge branch 'main'"))
        .await
        .unwrap()
        .unwrap();

    assert!(
        on_the_timeline(&pool, id).await[0].merge,
        "the card the Timeline draws is the one that says it is a merge",
    );

    assert!(
        commit(&pool, id, event).await.unwrap().unwrap().merge,
        "and so is the pane it opens",
    );
}

/// And the ordinary commit, which is every commit but the one a resolution
/// session left behind: nothing marks it, and the card is the one that has
/// always been drawn.
#[tokio::test]
async fn an_ordinary_commit_is_no_merge() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    let event = record_commit(&pool, id, repo, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap()
        .unwrap();

    assert!(!on_the_timeline(&pool, id).await[0].merge);
    assert!(!commit(&pool, id, event).await.unwrap().unwrap().merge);
}

/// One summary per commit, because the summary is written in the commit's own
/// transaction: a second sweep of the same commit is a sweep with nothing to do,
/// and cannot land a second row against the Event.
#[tokio::test]
async fn a_summary_is_recorded_once_with_its_commit() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    let written = summarised("a1b2c3d", "feat: rate limiting", "A bucket per account.");

    let event = record_commit(&pool, id, repo, &written)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        record_commit(&pool, id, repo, &written).await.unwrap(),
        None,
        "the second sweep finds nothing left to do",
    );

    assert_eq!(
        commit(&pool, id, event).await.unwrap(),
        Some(written),
        "and the summary is the one it was recorded with",
    );
}

/// The rule the whole design of this table is for. A sweep offers the branch
/// whole, so the second sweep offers everything the first one already recorded.
#[tokio::test]
async fn the_same_commit_offered_twice_is_recorded_once() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    let first = record_commit(&pool, id, repo, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap();
    let again = record_commit(&pool, id, repo, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap();

    assert!(first.is_some(), "the first sweep records it");
    assert_eq!(again, None, "and the second finds nothing left to do");

    assert_eq!(
        on_the_timeline(&pool, id).await.len(),
        1,
        "so the Timeline has it once",
    );
}

/// Two Conversations can be working on branches that share a commit — one
/// stacked on the other, or both branched from the same place — and each of
/// their Timelines is its own record.
#[tokio::test]
async fn one_commit_can_be_on_two_conversations() {
    let (_dir, pool) = fresh_pool().await;
    let (one, own) = conversation(&pool).await;

    let other = registered(&pool, "other").await;
    let two = start_conversation(&pool, other, "stacked")
        .await
        .unwrap()
        .unwrap();

    assert!(
        record_commit(&pool, one, own, &landed("a1b2c3d", "feat: rate limiting"))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        record_commit(&pool, two, other, &landed("a1b2c3d", "feat: rate limiting"))
            .await
            .unwrap()
            .is_some(),
        "the rule is one per Conversation, not one per repository",
    );

    assert_eq!(on_the_timeline(&pool, one).await.len(), 1);
    assert_eq!(on_the_timeline(&pool, two).await.len(), 1);
}

/// The order Events are read back in is the order they were recorded, which for
/// commits is the order they landed on the branch — a sweep offers them oldest
/// first.
#[tokio::test]
async fn commits_come_back_in_the_order_they_were_recorded() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    for (sha, subject) in [
        ("1111111", "test: a failing test"),
        ("2222222", "feat: rate limiting"),
        ("3333333", "docs: say what it does"),
    ] {
        record_commit(&pool, id, repo, &landed(sha, subject))
            .await
            .unwrap()
            .expect("each of these is new");
    }

    let subjects: Vec<String> = on_the_timeline(&pool, id)
        .await
        .into_iter()
        .map(|commit| commit.subject)
        .collect();

    assert_eq!(
        subjects,
        vec![
            "test: a failing test",
            "feat: rate limiting",
            "docs: say what it does"
        ],
    );
}

/// What a sweep asks before it goes reading git: everything already recorded is
/// a commit there is nothing left to do about.
#[tokio::test]
async fn what_is_already_recorded_can_be_asked_for_by_sha() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    assert_eq!(
        recorded_commits(&pool, id, repo).await.unwrap(),
        Vec::<String>::new(),
        "a Conversation that has committed nothing has nothing recorded",
    );

    record_commit(&pool, id, repo, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap();

    assert_eq!(
        recorded_commits(&pool, id, repo).await.unwrap(),
        vec!["a1b2c3d".to_owned()],
    );
}

/// A commit attributed to a Conversation that is not there would be on nobody's
/// Timeline, which is why the insert selects the Conversation rather than
/// trusting the id.
#[tokio::test]
async fn a_commit_on_no_conversation_is_not_recorded() {
    let (_dir, pool) = fresh_pool().await;
    let repo = registered(&pool, "verkstead").await;

    assert_eq!(
        record_commit(&pool, 404, repo, &landed("a1b2c3d", "feat: rate limiting"))
            .await
            .unwrap(),
        None,
    );
}

/// A commit is reached through the Timeline it is on. An Event id belonging to
/// another Conversation names nothing, which is what makes the details pane's
/// route conversation-scoped rather than only conversation-shaped.
#[tokio::test]
async fn a_commit_is_not_readable_through_another_conversation() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    let event = record_commit(&pool, id, repo, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(commit(&pool, id + 1, event).await.unwrap(), None);
}

/// The other half of *exactly once*: one commit per Conversation per Repo. A
/// Conversation is swept in its own repository and in each read-write
/// companion, and what a sha means is a fact about one repository — so the same
/// string out of two of them is two commits, and each lands.
#[tokio::test]
async fn a_sha_is_the_same_commit_only_within_one_repository() {
    let (_dir, pool) = fresh_pool().await;
    let (id, own) = conversation(&pool).await;

    let askance = registered(&pool, "askance").await;
    assert_eq!(
        add_companion(&pool, id, askance).await.unwrap(),
        verkstead_store::Adding::Added,
    );

    assert!(
        record_commit(&pool, id, own, &landed("a1b2c3d", "feat: rate limiting"))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        record_commit(
            &pool,
            id,
            askance,
            &landed("a1b2c3d", "feat: the other half")
        )
        .await
        .unwrap()
        .is_some(),
        "another repository's history is another history",
    );

    assert_eq!(
        on_the_timeline(&pool, id).await.len(),
        2,
        "so both are on the Timeline",
    );

    assert_eq!(
        record_commit(
            &pool,
            id,
            askance,
            &landed("a1b2c3d", "feat: the other half")
        )
        .await
        .unwrap(),
        None,
        "and the companion's next sweep still finds nothing left to do",
    );
}

/// What a sweep asks is what *its own* repository has recorded. A sha on the
/// companion's branch is nothing the Conversation's own sweep could act on, so
/// it must not come back as one it has already dealt with.
#[tokio::test]
async fn a_sweep_is_told_what_its_own_repository_has_recorded() {
    let (_dir, pool) = fresh_pool().await;
    let (id, own) = conversation(&pool).await;

    let askance = registered(&pool, "askance").await;
    add_companion(&pool, id, askance).await.unwrap();

    record_commit(
        &pool,
        id,
        askance,
        &landed("a1b2c3d", "feat: the other half"),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(
        recorded_commits(&pool, id, own).await.unwrap(),
        Vec::<String>::new(),
        "the Conversation's own repository has committed nothing",
    );
    assert_eq!(
        recorded_commits(&pool, id, askance).await.unwrap(),
        vec!["a1b2c3d".to_owned()],
    );
}

/// The label: a companion's commit says which repository it came out of, and
/// the Conversation's own says nothing, because an unlabeled card means the
/// work's own repo.
#[tokio::test]
async fn a_companion_repos_commit_is_labelled_and_the_conversations_own_is_not() {
    let (_dir, pool) = fresh_pool().await;
    let (id, own) = conversation(&pool).await;

    let askance = registered(&pool, "askance").await;
    add_companion(&pool, id, askance).await.unwrap();

    let ours = record_commit(&pool, id, own, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap()
        .unwrap();
    let theirs = record_commit(
        &pool,
        id,
        askance,
        &landed("9f8e7d6", "feat: the other half"),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(commit(&pool, id, ours).await.unwrap().unwrap().repo, None);
    assert_eq!(
        commit(&pool, id, theirs).await.unwrap().unwrap().repo,
        Some("askance".to_owned()),
        "the Repo's registered name, which is what the card draws",
    );

    let drawn: Vec<Option<String>> = on_the_timeline(&pool, id)
        .await
        .into_iter()
        .map(|commit| commit.repo)
        .collect();

    assert_eq!(
        drawn,
        vec![None, Some("askance".to_owned())],
        "and the Timeline says the same thing the pane does",
    );
}

/// Which repository the details pane reads a commit's diff out of: the one it
/// was recorded against. A companion's commit is in the companion's repository,
/// and the Conversation's own would know nothing about it.
#[tokio::test]
async fn a_commit_says_which_repository_to_read_it_out_of() {
    let (_dir, pool) = fresh_pool().await;
    let (id, own) = conversation(&pool).await;

    let askance = registered(&pool, "askance").await;
    add_companion(&pool, id, askance).await.unwrap();

    let ours = record_commit(&pool, id, own, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap()
        .unwrap();
    let theirs = record_commit(
        &pool,
        id,
        askance,
        &landed("9f8e7d6", "feat: the other half"),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(
        commit_repo(&pool, id, ours).await.unwrap().unwrap().path,
        Path::new("/watched/verkstead"),
    );
    assert_eq!(
        commit_repo(&pool, id, theirs).await.unwrap().unwrap().path,
        Path::new("/watched/askance"),
    );

    assert_eq!(
        commit_repo(&pool, id + 1, theirs).await.unwrap(),
        None,
        "and it is reached through the Timeline it is on, like the commit itself",
    );
}

/// The other half of a sweep. A branch whose commits have been rewritten —
/// rebased to settle a conflict, or amended — carries the same work under new
/// shas, and what the Timeline holds under the old ones is taken off it.
///
/// Everything hanging off the Event goes with it: the commit row the Timeline
/// draws from, and the Commit Summary the details pane renders above the diff.
#[tokio::test]
async fn a_forgotten_commit_takes_its_event_and_its_summary_with_it() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    let written = summarised("a1b2c3d", "feat: rate limiting", "A bucket per account.");

    let event = record_commit(&pool, id, repo, &written)
        .await
        .unwrap()
        .unwrap();

    let kept = record_commit(&pool, id, repo, &landed("9f8e7d6", "feat: the other half"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        forget_commit(&pool, id, repo, "a1b2c3d").await.unwrap(),
        Some(event),
        "the Event it was, which is what the sweep logs",
    );

    assert_eq!(
        on_the_timeline(&pool, id).await,
        vec![landed("9f8e7d6", "feat: the other half")],
        "the Timeline is left with the commit the branch still carries",
    );
    assert_eq!(
        commit(&pool, id, event).await.unwrap(),
        None,
        "and the commit row it was drawn from has gone with its Event",
    );
    assert!(
        commit(&pool, id, kept).await.unwrap().is_some(),
        "which is the one Event and not the Timeline",
    );

    // The summary is what would be left behind: it hangs off the Event by id,
    // so a row surviving it would attach itself to whatever Event was written
    // next.
    let recorded = record_commit(&pool, id, repo, &landed("1a2b3c4", "feat: rate limiting"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        commit(&pool, id, recorded).await.unwrap().unwrap().summary,
        None,
        "the forgotten commit's summary went with it",
    );
}

/// A commit that is not there is nothing to forget, which is the answer
/// `record_commit` gives for one that already is: a sweep offers the whole of
/// what it read, and being wrong about either costs nothing.
#[tokio::test]
async fn forgetting_a_commit_that_is_not_there_is_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    record_commit(&pool, id, repo, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        forget_commit(&pool, id, repo, "9f8e7d6").await.unwrap(),
        None,
        "a sha this Conversation never recorded",
    );
    assert!(
        forget_commit(&pool, id, repo, "a1b2c3d")
            .await
            .unwrap()
            .is_some(),
        "the one it did",
    );
    assert_eq!(
        forget_commit(&pool, id, repo, "a1b2c3d").await.unwrap(),
        None,
        "and forgetting it a second time is nothing either",
    );
}

/// A sha is the commit's identity only with the Conversation and the Repo beside
/// it: two repositories are two histories, and two Conversations off one branch
/// hold the same shas.
#[tokio::test]
async fn forgetting_one_conversations_commit_leaves_anothers_alone() {
    let (_dir, pool) = fresh_pool().await;
    let (id, own) = conversation(&pool).await;

    let askance = registered(&pool, "askance").await;
    add_companion(&pool, id, askance).await.unwrap();

    record_commit(&pool, id, own, &landed("a1b2c3d", "feat: rate limiting"))
        .await
        .unwrap()
        .unwrap();
    record_commit(
        &pool,
        id,
        askance,
        &landed("a1b2c3d", "feat: the other half"),
    )
    .await
    .unwrap()
    .unwrap();

    forget_commit(&pool, id, own, "a1b2c3d").await.unwrap();

    assert_eq!(
        recorded_commits(&pool, id, own).await.unwrap(),
        Vec::<String>::new(),
    );
    assert_eq!(
        recorded_commits(&pool, id, askance).await.unwrap(),
        vec!["a1b2c3d".to_owned()],
        "the companion's commit is another repository's history",
    );
}

/// Where a Conversation's commits stand is what the runner reads before a
/// session and again after it, and it has to move for work a sweep *replaced*
/// rather than added.
///
/// A branch that was rebased or amended carries the same work under new shas, so
/// the sweep forgets as many commits as it records. Counted, that comes back to
/// the number it started at and the session reads as one that committed nothing
/// — which is exactly the resolution session on a Repo that rebases.
#[tokio::test]
async fn where_commits_stand_moves_when_a_sweep_replaces_one() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    assert_eq!(
        commits_landed(&pool, id).await.unwrap(),
        0,
        "a Conversation with nothing on its Timeline stands at nothing",
    );

    record_commit(&pool, id, repo, &landed("a1b2c3d", "feat: rate limitting"))
        .await
        .unwrap()
        .unwrap();

    let before = commits_landed(&pool, id).await.unwrap();
    assert!(before > 0, "and one with a commit on it stands past that");

    // The amend, as a sweep does it: the sha the branch stopped carrying comes
    // off, and the one it carries now goes on. One commit either side of it.
    forget_commit(&pool, id, repo, "a1b2c3d").await.unwrap();
    record_commit(&pool, id, repo, &landed("9f8e7d6", "feat: rate limiting"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        on_the_timeline(&pool, id).await.len(),
        1,
        "the Timeline holds what it held, which is what a count would read",
    );
    assert!(
        commits_landed(&pool, id).await.unwrap() > before,
        "and where those commits stand has moved, because the sweep recorded",
    );
}

/// And a sweep that only forgets moves it the other way, which is a session that
/// committed nothing however far the Timeline has been rewound.
#[tokio::test]
async fn where_commits_stand_never_reads_a_reset_as_a_commit() {
    let (_dir, pool) = fresh_pool().await;
    let (id, repo) = conversation(&pool).await;

    for (sha, subject) in [("a1b2c3d", "feat: one"), ("9f8e7d6", "feat: two")] {
        record_commit(&pool, id, repo, &landed(sha, subject))
            .await
            .unwrap()
            .unwrap();
    }

    let before = commits_landed(&pool, id).await.unwrap();

    forget_commit(&pool, id, repo, "9f8e7d6").await.unwrap();

    assert!(
        commits_landed(&pool, id).await.unwrap() < before,
        "a branch reset to where it was is nothing committed",
    );
}
