//! The pull request a Conversation's work was carried to, and the move into
//! Wrapping that recording one is.
//!
//! Recording the PR is not a thing that happens beside the move — it *is* the
//! move, in one transaction. So what these ask is what a Conversation and its
//! Timeline say afterwards: the state, the PR Event, and the move under it.
//!
//! Two states get here, because two kinds of work end on a pull request: a
//! backlog worked to empty, from Implementing, and a roadmap, from Grilling.
//!
//! And a Conversation can arrive twice. A review that split its findings out
//! into a backlog sends the work back to be built, and its finish step wraps up
//! again on the pull request it already had.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Event, Finished, Lifecycle, Merging, PullRequest, Rebuilding, Resolving, Rollup, Standing,
    WAITED_ON, WaitingOn, Wrapping, check_rollup, close_conversation, finish_wrap_up,
    implement_again, load_conversation, merges, merging, open_database, pick_direction,
    pull_request, pull_request_repo, pull_requests, record_another_pull_request,
    record_check_rollup, record_merging, record_pull_request, record_standing, register_repo,
    resolve_conflicts, save_brief, settle_wrap_up, standing, start_conversation, start_grilling,
    timeline, unfinished_pull_requests, wrap_up_settled,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation that is grilling, which is where every direction is picked and
/// where a roadmap Conversation still is when its pull request opens.
///
/// Walked there rather than moved by hand: every state on the way records
/// something — the base commit, the worktree — and a Conversation dropped
/// straight into one would be one nothing else in the store agrees about.
async fn grilling(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let id = start_conversation(pool, repo.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();
    start_grilling(
        pool,
        id,
        "c0ffee",
        Path::new("/state/worktrees/rate-limiting"),
        &[],
    )
    .await
    .unwrap();

    id
}

/// And the same one carried on to having its work built, which is where a finish
/// step reaches one. Inline, because that is the direction whose pick makes the
/// move.
async fn implementing(pool: &SqlitePool) -> i64 {
    let id = grilling(pool).await;

    pick_direction(pool, id, verkstead_schema::Direction::Inline)
        .await
        .unwrap();

    id
}

/// The PR the finish step opened, as the host's `gh` read it back.
fn opened() -> PullRequest {
    PullRequest {
        number: 41,
        title: "Rate limiting".to_owned(),
        url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
        repo: None,
    }
}

/// The Repo a Conversation's own work is in, which is what its own pull request
/// is recorded against.
async fn own(pool: &SqlitePool, id: i64) -> i64 {
    load_conversation(pool, id).await.unwrap().unwrap().repo.id
}

/// And another registered Repo, which is what a read-write companion's pull
/// request is recorded against.
async fn companion(pool: &SqlitePool) -> i64 {
    register_repo(pool, Path::new("/srv/askance"), "askance", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet")
        .id
}

/// Everything a wrap-up waits on, read off the record rather than written out —
/// which is what walking a Conversation to Done here means settling.
async fn waiting_on(pool: &SqlitePool, id: i64) -> Vec<WaitingOn> {
    let opened = pull_requests(pool, id).await.unwrap();

    WAITED_ON
        .into_iter()
        .chain(opened.into_iter().flat_map(|(repo, _)| {
            [
                WaitingOn::Checks(repo.id),
                WaitingOn::Comments(repo.id),
                WaitingOn::Mergeable(repo.id),
            ]
        }))
        .collect()
}

/// What a Conversation's Timeline holds, in order.
async fn events(pool: &SqlitePool, id: i64) -> Vec<Event> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .map(|event| event.event)
        .collect()
}

/// The whole of it: the PR is recorded, the Conversation is wrapping, and both
/// the PR and the move are on the Timeline.
#[tokio::test]
async fn recording_the_pull_request_moves_the_conversation_into_wrapping() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;

    assert_eq!(
        record_pull_request(&pool, id, own(&pool, id).await, &opened())
            .await
            .unwrap(),
        Wrapping::Started,
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Wrapping);

    let events = events(&pool, id).await;

    assert_eq!(
        events.last(),
        Some(&Event::Moved(Lifecycle::Wrapping)),
        "the move is the last thing on the Timeline: {events:?}",
    );
    assert_eq!(
        events[events.len() - 2],
        Event::PullRequest(opened()),
        "and the PR is what it moved on: {events:?}",
    );
}

/// A roadmap Conversation gets here from Grilling, with no Implementing on the
/// way: the session that settled the work wrote the roadmap and carried the
/// branch to a pull request without ever leaving the grilling, because the
/// building belongs to the Stages it planned.
#[tokio::test]
async fn a_roadmap_conversation_wraps_up_straight_out_of_its_grilling() {
    let (_dir, pool) = fresh_pool().await;
    let id = grilling(&pool).await;

    pick_direction(&pool, id, verkstead_schema::Direction::Roadmap)
        .await
        .unwrap();

    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Grilling,
        "the pick moved nothing: the grilling is what writes the roadmap",
    );

    assert_eq!(
        record_pull_request(&pool, id, own(&pool, id).await, &opened())
            .await
            .unwrap(),
        Wrapping::Started,
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Wrapping);

    assert_eq!(
        events(&pool, id)
            .await
            .into_iter()
            .filter_map(|event| match event {
                Event::Moved(state) => Some(state),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [Lifecycle::Grilling, Lifecycle::Wrapping],
        "and the ladder skips Implementing rather than idling in it",
    );
}

/// A second attempt at the same finish finds the move already made. Nothing is
/// recorded twice — the state check is what says so, and it is read inside the
/// same transaction the insert is in.
#[tokio::test]
async fn a_conversation_that_is_already_wrapping_records_no_second_pull_request() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;

    record_pull_request(&pool, id, own(&pool, id).await, &opened())
        .await
        .unwrap();

    assert_eq!(
        record_pull_request(&pool, id, own(&pool, id).await, &opened())
            .await
            .unwrap(),
        Wrapping::NothingToWrap,
    );

    let requests = events(&pool, id)
        .await
        .into_iter()
        .filter(|event| matches!(event, Event::PullRequest(_)))
        .count();

    assert_eq!(requests, 1, "one Conversation, one pull request");
}

/// A Conversation closed out from under the run is not one to move into
/// Wrapping, however far the session it was running had got.
#[tokio::test]
async fn a_closed_conversation_is_not_moved_on_by_a_pull_request() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;

    close_conversation(&pool, id).await.unwrap();

    assert_eq!(
        record_pull_request(&pool, id, own(&pool, id).await, &opened())
            .await
            .unwrap(),
        Wrapping::NothingToWrap,
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Closed);
}

#[tokio::test]
async fn a_conversation_that_is_not_there_records_nothing() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        record_pull_request(&pool, 404, 1, &opened()).await.unwrap(),
        Wrapping::NoSuchConversation,
    );
}

/// A second wrap records nothing new. The Conversation left Wrapping to build a
/// backlog its review split out, and what its finish step opened is the pull
/// request it already had — so the record is reused, and what says the work came
/// round again is the lifecycle moves either side of it.
#[tokio::test]
async fn a_second_wrap_reuses_the_pull_request_the_first_one_recorded() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;

    record_pull_request(&pool, id, own(&pool, id).await, &opened())
        .await
        .unwrap();

    assert_eq!(
        implement_again(&pool, id).await.unwrap(),
        Rebuilding::Started,
    );
    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Implementing,
        "the split-out work is built, and building is Implementing",
    );

    assert_eq!(
        record_pull_request(&pool, id, own(&pool, id).await, &opened())
            .await
            .unwrap(),
        Wrapping::Started,
        "and the finish that follows the backlog wraps it up again",
    );

    let events = events(&pool, id).await;

    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::PullRequest(_)))
            .count(),
        1,
        "one Conversation, one pull request, however many times it wraps up: {events:?}",
    );
    assert_eq!(
        events
            .into_iter()
            .filter_map(|event| match event {
                Event::Moved(state) => Some(state),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [
            Lifecycle::Grilling,
            Lifecycle::Wrapping,
            Lifecycle::Implementing,
            Lifecycle::Wrapping,
        ],
        "and the moves are what tell the re-entry's story",
    );
    assert_eq!(
        pull_request(&pool, id, own(&pool, id).await).await.unwrap(),
        Some(opened()),
        "with the one record still the one the watchers read",
    );
}

/// A Conversation that is not wrapping up has no wrap-up to leave, whether it is
/// being built already or was closed out from under the session that would have
/// split the work out.
#[tokio::test]
async fn only_a_wrapping_conversation_can_be_sent_back_to_be_built() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;

    assert_eq!(
        implement_again(&pool, id).await.unwrap(),
        Rebuilding::NotWrapping,
    );

    record_pull_request(&pool, id, own(&pool, id).await, &opened())
        .await
        .unwrap();
    close_conversation(&pool, id).await.unwrap();

    assert_eq!(
        implement_again(&pool, id).await.unwrap(),
        Rebuilding::NotWrapping,
    );
    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Closed,
    );

    assert_eq!(
        implement_again(&pool, 404).await.unwrap(),
        Rebuilding::NoSuchConversation,
    );
}

/// A Conversation ends on one pull request per repository it was worked in. The
/// work's own moves it into Wrapping; a read-write companion's is that same
/// wrap-up learning about another pull request, and moves nothing.
#[tokio::test]
async fn a_companions_pull_request_stands_beside_the_works_own() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    let beside = companion(&pool).await;

    record_pull_request(&pool, id, own(&pool, id).await, &opened())
        .await
        .unwrap();

    assert!(
        record_another_pull_request(&pool, id, beside, &beside_it())
            .await
            .unwrap(),
        "the companion's pull request is recorded against the Conversation",
    );

    assert_eq!(
        pull_request(&pool, id, own(&pool, id).await).await.unwrap(),
        Some(opened()),
        "the work's own reads back unlabeled",
    );
    assert_eq!(
        pull_request(&pool, id, beside).await.unwrap(),
        Some(PullRequest {
            repo: Some("askance".to_owned()),
            ..beside_it()
        }),
        "and the companion's reads back named with its repository",
    );

    let requests: Vec<Event> = events(&pool, id)
        .await
        .into_iter()
        .filter(|event| matches!(event, Event::PullRequest(_)))
        .collect();

    assert_eq!(requests.len(), 2, "both are on the Timeline: {requests:?}",);
    assert_eq!(
        events(&pool, id)
            .await
            .into_iter()
            .filter_map(|event| match event {
                Event::Moved(state) => Some(state),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [Lifecycle::Grilling, Lifecycle::Wrapping],
        "and the second one moved nothing twice",
    );
}

/// The pull request a companion's branch was carried to, which is a different
/// repository's number entirely — `#41` there is somebody else's work.
fn beside_it() -> PullRequest {
    PullRequest {
        number: 7,
        title: "Rate limiting".to_owned(),
        url: "https://github.com/tobico/askance/pull/7".to_owned(),
        repo: None,
    }
}

/// A pull request recorded against a repository that already has one reuses the
/// row it has. Which is what makes a discovery that runs twice do nothing the
/// second time — and what a second wrap lands on.
#[tokio::test]
async fn a_repository_that_already_has_one_keeps_the_row_it_has() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    let beside = companion(&pool).await;

    record_pull_request(&pool, id, own(&pool, id).await, &opened())
        .await
        .unwrap();
    record_another_pull_request(&pool, id, beside, &beside_it())
        .await
        .unwrap();
    record_another_pull_request(&pool, id, beside, &beside_it())
        .await
        .unwrap();

    let requests = events(&pool, id)
        .await
        .into_iter()
        .filter(|event| matches!(event, Event::PullRequest(_)))
        .count();

    assert_eq!(requests, 2, "one pull request per repository, and no more");
}

/// A Conversation that is not there has nothing to record another pull request
/// against, which is the one thing this is refused for.
#[tokio::test]
async fn another_pull_request_needs_a_conversation_to_stand_on() {
    let (_dir, pool) = fresh_pool().await;

    assert!(
        !record_another_pull_request(&pool, 404, 1, &opened())
            .await
            .unwrap(),
    );
}

/// The details pane asks GitHub in the repository the pull request was opened
/// in, which for a companion's is not the Conversation's own.
#[tokio::test]
async fn a_pull_request_says_which_repository_it_is_in() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    let beside = companion(&pool).await;

    record_pull_request(&pool, id, own(&pool, id).await, &opened())
        .await
        .unwrap();
    record_another_pull_request(&pool, id, beside, &beside_it())
        .await
        .unwrap();

    let where_they_are: Vec<(i64, Option<String>)> = {
        let mut found = Vec::new();

        for event in timeline(&pool, id).await.unwrap() {
            if !matches!(event.event, Event::PullRequest(_)) {
                continue;
            }

            found.push((
                event.id,
                pull_request_repo(&pool, id, event.id)
                    .await
                    .unwrap()
                    .map(|repo| repo.path.to_string_lossy().into_owned()),
            ));
        }

        found
    };

    assert_eq!(
        where_they_are
            .iter()
            .map(|(_, path)| path.as_deref())
            .collect::<Vec<_>>(),
        [Some("/srv/verkstead"), Some("/srv/askance")],
        "each is asked about in the repository it belongs to",
    );

    let elsewhere = where_they_are[0].0;

    assert_eq!(
        pull_request_repo(&pool, 404, elsewhere).await.unwrap(),
        None,
        "and an Event of another Conversation's names nothing here",
    );
}

/// How the checks on it are, which is the one thing about a pull request that is
/// written down and moves.
///
/// Written on every poll of the watcher and read by the Conversation view, so
/// what these ask is the two things the card depends on: that the last word
/// written is the word read back, and that saying the same thing twice is not
/// news.
#[tokio::test]
async fn how_the_checks_are_is_written_down_and_read_back() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    record_pull_request(&pool, id, own(&pool, id).await, &opened())
        .await
        .unwrap();

    assert_eq!(
        check_rollup(&pool, id).await.unwrap(),
        None,
        "nothing has asked GitHub yet, which is not the same as green",
    );

    assert!(
        record_check_rollup(&pool, id, Rollup::Running)
            .await
            .unwrap(),
        "the first poll is news",
    );
    assert_eq!(
        check_rollup(&pool, id).await.unwrap(),
        Some(Rollup::Running)
    );

    assert!(
        !record_check_rollup(&pool, id, Rollup::Running)
            .await
            .unwrap(),
        "and a suite still running half an hour later is the same thing said again",
    );

    assert!(
        record_check_rollup(&pool, id, Rollup::Failed)
            .await
            .unwrap(),
        "a check going red is news",
    );
    assert_eq!(check_rollup(&pool, id).await.unwrap(), Some(Rollup::Failed));

    assert!(
        record_check_rollup(&pool, id, Rollup::Passed)
            .await
            .unwrap(),
        "and so is the fix session's push going green",
    );
    assert_eq!(check_rollup(&pool, id).await.unwrap(), Some(Rollup::Passed));
}

/// And it survives a restart, which is the whole reason it is written down
/// rather than held in the watcher: the watching stops when the wrap-up is over,
/// and the card on a Done Conversation goes on drawing what the last poll found.
#[tokio::test]
async fn how_the_checks_are_outlives_the_server_that_asked() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let pool = open_database(&database).await.unwrap();
    let id = implementing(&pool).await;
    record_pull_request(&pool, id, own(&pool, id).await, &opened())
        .await
        .unwrap();
    record_check_rollup(&pool, id, Rollup::Passed)
        .await
        .unwrap();
    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    assert_eq!(check_rollup(&pool, id).await.unwrap(), Some(Rollup::Passed));
}

/// And whether it merges, which is the other reading of GitHub written down
/// here — kept per pull request rather than per Conversation, because a conflict
/// is a fact about one branch and its base.
///
/// A Conversation with a read-write companion has one clean and one conflicted
/// as easily as two of either: the base moved in one repository and not in the
/// other. So what these ask is that the two are told apart, that the last word
/// written is the word read back, and — as for the rollup beside it — that
/// saying the same thing twice is not news.
#[tokio::test]
async fn whether_each_pull_request_merges_is_written_down_and_read_back() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    let beside = companion(&pool).await;

    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();
    record_another_pull_request(&pool, id, beside, &beside_it())
        .await
        .unwrap();

    assert_eq!(
        merging(&pool, id, own).await.unwrap(),
        None,
        "nothing has asked GitHub yet, which is not the same as merging cleanly",
    );

    assert!(
        record_merging(&pool, id, own, Merging::Conflicting)
            .await
            .unwrap(),
        "the first poll is news",
    );
    record_merging(&pool, id, beside, Merging::Cleanly)
        .await
        .unwrap();

    assert_eq!(
        merging(&pool, id, own).await.unwrap(),
        Some(Merging::Conflicting),
    );
    assert_eq!(
        merging(&pool, id, beside).await.unwrap(),
        Some(Merging::Cleanly),
        "the companion's own base has not moved, and its pull request says so",
    );

    assert!(
        !record_merging(&pool, id, own, Merging::Conflicting)
            .await
            .unwrap(),
        "and a conflict still standing on the next poll is the same thing said again",
    );

    // And the conflict resolved: written over rather than added to, a conflict
    // that has been dealt with not being a conflict.
    assert!(
        record_merging(&pool, id, own, Merging::Cleanly)
            .await
            .unwrap(),
        "a resolution landing is news, which is what takes the mark off the card",
    );

    assert_eq!(
        merging(&pool, id, own).await.unwrap(),
        Some(Merging::Cleanly)
    );
}

/// And every one of them together, by the Timeline Event each pull request is,
/// which is what the Conversation view draws its cards off.
///
/// Keyed by the Event rather than by the Repo because that is what a card has to
/// hand — the same pull request is drawn pinned above the record and at the
/// moment it opened, and both copies know only which Event they are. A pull
/// request nothing has asked GitHub about is missing from the map rather than
/// carried as a word, which is the card that draws no mark.
#[tokio::test]
async fn every_pull_requests_merge_is_read_back_by_the_event_it_is() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    let beside = companion(&pool).await;

    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();
    record_another_pull_request(&pool, id, beside, &beside_it())
        .await
        .unwrap();

    assert!(
        merges(&pool, id).await.unwrap().is_empty(),
        "nothing has asked GitHub about either of them, so there is nothing to draw",
    );

    record_merging(&pool, id, own, Merging::Conflicting)
        .await
        .unwrap();

    // Which Event each pull request is, in the order they were recorded — the
    // Conversation's own first, then the companion's.
    let events: Vec<i64> = timeline(&pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| matches!(event.event, Event::PullRequest(_)))
        .map(|event| event.id)
        .collect();

    assert_eq!(
        merges(&pool, id).await.unwrap(),
        std::collections::HashMap::from([(events[0], Merging::Conflicting)]),
        "the one that was asked about is in it, and the one that was not is absent",
    );

    record_merging(&pool, id, beside, Merging::Cleanly)
        .await
        .unwrap();

    assert_eq!(
        merges(&pool, id).await.unwrap(),
        std::collections::HashMap::from([
            (events[0], Merging::Conflicting),
            (events[1], Merging::Cleanly),
        ]),
        "and each Event carries what was written down about its own pull request",
    );
}

/// And it survives a restart, for the reason the rollup beside it does: the
/// watching stops when the wrap-up is over, and what is drawn on a Done
/// Conversation afterwards is the last thing anybody asked GitHub.
#[tokio::test]
async fn whether_a_pull_request_merges_outlives_the_server_that_asked() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let pool = open_database(&database).await.unwrap();
    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();
    record_merging(&pool, id, own, Merging::Conflicting)
        .await
        .unwrap();
    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    assert_eq!(
        merging(&pool, id, own).await.unwrap(),
        Some(Merging::Conflicting),
    );
}

/// Where a pull request has got to is written down beside whether it merges,
/// per pull request and the same way.
///
/// A different fact from the merge rather than a qualifier on it: a pull request
/// somebody has merged still merged cleanly, and the two are read out of one
/// `gh` answer into two rows. So what this asks is that the two are told apart,
/// that a companion's is its own, and that the last word written is the word
/// read back.
#[tokio::test]
async fn where_each_pull_request_has_got_to_is_written_down_and_read_back() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    let beside = companion(&pool).await;

    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();
    record_another_pull_request(&pool, id, beside, &beside_it())
        .await
        .unwrap();

    assert_eq!(
        standing(&pool, id, own).await.unwrap(),
        None,
        "nothing has asked GitHub yet, which is not the same as being open",
    );

    record_standing(&pool, id, own, Standing::Open)
        .await
        .unwrap();
    record_standing(&pool, id, beside, Standing::Merged)
        .await
        .unwrap();

    assert_eq!(
        standing(&pool, id, own).await.unwrap(),
        Some(Standing::Open)
    );
    assert_eq!(
        standing(&pool, id, beside).await.unwrap(),
        Some(Standing::Merged),
        "the companion's half has landed and the work's own has not, which is two \
         repositories being two repositories",
    );

    // And the merge beside it is untouched by any of that: a pull request that
    // has been merged merged cleanly.
    record_merging(&pool, id, own, Merging::Conflicting)
        .await
        .unwrap();

    assert_eq!(
        standing(&pool, id, own).await.unwrap(),
        Some(Standing::Open)
    );
    assert_eq!(
        merging(&pool, id, own).await.unwrap(),
        Some(Merging::Conflicting),
    );

    // Written over rather than added to, as every reading of GitHub here is.
    record_standing(&pool, id, own, Standing::Closed)
        .await
        .unwrap();

    assert_eq!(
        standing(&pool, id, own).await.unwrap(),
        Some(Standing::Closed),
    );
}

/// Which pull requests are still worth asking GitHub about: a Done
/// Conversation's, that nothing has recorded merged or closed.
///
/// The whole of what the sweep after Done walks. A Conversation still wrapping
/// up has a watcher of its own asking every half minute, so it is not here; a
/// Closed one is the human finished with the work, so it is not here either —
/// and an Archived one is a Closed one off the sidebar, which is the same answer
/// by the same route.
#[tokio::test]
async fn the_pull_requests_still_waiting_to_land_are_the_done_ones_nobody_has_merged() {
    let (_dir, pool) = fresh_pool().await;

    let wrapping = implementing(&pool).await;
    let own = own(&pool, wrapping).await;
    record_pull_request(&pool, wrapping, own, &opened())
        .await
        .unwrap();

    assert_eq!(
        unfinished_pull_requests(&pool).await.unwrap(),
        Vec::new(),
        "a wrap-up's own watcher is asking about this every half minute already",
    );

    // The work finishes, which is where the watching stops and the sweeping
    // starts.
    for waiting_on in waiting_on(&pool, wrapping).await {
        settle_wrap_up(&pool, wrapping, waiting_on).await.unwrap();
    }
    assert_eq!(
        finish_wrap_up(&pool, wrapping).await.unwrap(),
        Finished::Done
    );

    assert_eq!(
        unfinished_pull_requests(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|unfinished| (
                unfinished.conversation_id,
                unfinished.repo.id,
                unfinished.number
            ))
            .collect::<Vec<_>>(),
        [(wrapping, own, 41)],
        "and the Repo comes with it, `gh` reading its repository from wherever it \
         is run",
    );

    // A reading that leaves it open leaves it on the list, which is the sweep
    // going on asking.
    record_standing(&pool, wrapping, own, Standing::Open)
        .await
        .unwrap();

    assert_eq!(unfinished_pull_requests(&pool).await.unwrap().len(), 1);

    // And one that says somebody has merged it takes it off for good.
    record_standing(&pool, wrapping, own, Standing::Merged)
        .await
        .unwrap();

    assert_eq!(
        unfinished_pull_requests(&pool).await.unwrap(),
        Vec::new(),
        "a merged pull request is a question with a final answer",
    );
}

/// And a Closed Conversation's pull request is never on the list, however open
/// it is.
#[tokio::test]
async fn a_closed_conversations_pull_request_is_never_asked_about() {
    let (_dir, pool) = fresh_pool().await;

    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();

    for waiting_on in waiting_on(&pool, id).await {
        settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }
    assert_eq!(finish_wrap_up(&pool, id).await.unwrap(), Finished::Done);
    assert_eq!(unfinished_pull_requests(&pool).await.unwrap().len(), 1);

    close_conversation(&pool, id).await.unwrap();

    assert_eq!(
        unfinished_pull_requests(&pool).await.unwrap(),
        Vec::new(),
        "closing is the human finished with the work, and nothing goes on \
         watching what they are finished with",
    );
}

/// And what has been written down about a pull request outlives the server that
/// asked, for the reason everything beside it does: a sweep every fifteen
/// minutes is not what a card drawn an hour later is read off.
#[tokio::test]
async fn where_a_pull_request_has_got_to_outlives_the_server_that_asked() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let pool = open_database(&database).await.unwrap();
    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();
    record_standing(&pool, id, own, Standing::Merged)
        .await
        .unwrap();
    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    assert_eq!(
        standing(&pool, id, own).await.unwrap(),
        Some(Standing::Merged),
    );
}

/// The press that sends a Done Conversation back to a wrap-up, because the pull
/// request it finished on has since stopped merging.
///
/// The whole of what the move writes: the state, the human's own line above the
/// machine's move, and the merge back to being something the wrap-up waits on —
/// with the review's settle deliberately left exactly where it was, which is the
/// difference between this press and a steer into Wrapping.
#[tokio::test]
async fn resolving_a_conflict_sends_a_done_conversation_back_to_wrapping_up() {
    let (_dir, pool) = fresh_pool().await;

    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();

    for waiting_on in waiting_on(&pool, id).await {
        settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }
    assert_eq!(finish_wrap_up(&pool, id).await.unwrap(), Finished::Done);

    // The base moved under the branch while nobody was working on it, which is
    // what the sweep after Done writes down and dispatches nothing about.
    record_merging(&pool, id, own, Merging::Conflicting)
        .await
        .unwrap();

    assert_eq!(
        resolve_conflicts(&pool, id).await.unwrap(),
        Resolving::Wrapping,
    );

    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Wrapping,
    );

    assert_eq!(
        events(&pool, id)
            .await
            .into_iter()
            .filter_map(|event| match event {
                Event::Steer(target, said) => Some(Ok((target, said))),
                Event::Moved(state) => Some(Err(state)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [
            Err(Lifecycle::Grilling),
            Err(Lifecycle::Wrapping),
            Err(Lifecycle::Done),
            // The press, in a steer's own shape and carrying nothing written:
            // somebody decided this, and the move under it is what came of it.
            Ok((Lifecycle::Wrapping, None)),
            Err(Lifecycle::Wrapping),
        ],
        "the human's line lands above the machine's move",
    );

    let settled = wrap_up_settled(&pool, id).await.unwrap();

    assert!(
        !settled.contains(&WaitingOn::Mergeable(own)),
        "the conflict the press was made over is something the wrap-up waits on \
         again: {settled:?}",
    );
    assert!(
        settled.contains(&WaitingOn::Review),
        "and the review it was carried to Done on stands, so nothing reads the \
         branch a second time: {settled:?}",
    );
    assert!(
        settled.contains(&WaitingOn::Checks(own)) && settled.contains(&WaitingOn::Comments(own)),
        "and so does everything this round's own polls settle for themselves: \
         {settled:?}",
    );

    assert_eq!(
        finish_wrap_up(&pool, id).await.unwrap(),
        Finished::StillWaiting,
        "so the wrap-up that starts here does not finish on the first turn of \
         its settling loop",
    );
}

/// Only the pull requests the record says conflict go back to being waited on.
///
/// A Conversation ends on one per repository it was worked in, and a base that
/// moved in one of them did not move in the other — so a companion that still
/// merges keeps its settle, and the wrap-up waits on the one branch that has
/// stopped merging rather than on all of them.
#[tokio::test]
async fn only_the_pull_requests_that_conflict_go_back_to_being_waited_on() {
    let (_dir, pool) = fresh_pool().await;

    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    let beside = companion(&pool).await;
    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();
    record_another_pull_request(&pool, id, beside, &beside_it())
        .await
        .unwrap();

    for waiting_on in waiting_on(&pool, id).await {
        settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }
    assert_eq!(finish_wrap_up(&pool, id).await.unwrap(), Finished::Done);

    record_merging(&pool, id, own, Merging::Cleanly)
        .await
        .unwrap();
    record_merging(&pool, id, beside, Merging::Conflicting)
        .await
        .unwrap();

    assert_eq!(
        resolve_conflicts(&pool, id).await.unwrap(),
        Resolving::Wrapping,
    );

    let settled = wrap_up_settled(&pool, id).await.unwrap();

    assert!(
        !settled.contains(&WaitingOn::Mergeable(beside)),
        "the companion's branch is the one that stopped merging: {settled:?}",
    );
    assert!(
        settled.contains(&WaitingOn::Mergeable(own)),
        "and the work's own still merges, so nothing about it was unsettled: \
         {settled:?}",
    );
}

/// A press made where nothing conflicts is refused rather than made.
///
/// The button is drawn off the recorded fact, so this is a press against a
/// reading that has moved on — somebody else resolved it, or the freshening the
/// pane does as it opens found the conflict gone. Putting the Conversation back
/// to Wrapping for it would be a round trip to Done with nothing done on the
/// way.
#[tokio::test]
async fn a_conversation_with_nothing_conflicting_is_left_where_it_is() {
    let (_dir, pool) = fresh_pool().await;

    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();

    for waiting_on in waiting_on(&pool, id).await {
        settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }
    assert_eq!(finish_wrap_up(&pool, id).await.unwrap(), Finished::Done);

    // Nothing has asked GitHub at all, which is not a conflict — not knowing
    // never is.
    assert_eq!(
        resolve_conflicts(&pool, id).await.unwrap(),
        Resolving::NothingConflicts,
    );

    record_merging(&pool, id, own, Merging::Cleanly)
        .await
        .unwrap();

    assert_eq!(
        resolve_conflicts(&pool, id).await.unwrap(),
        Resolving::NothingConflicts,
        "and a pull request GitHub says merges is not one either",
    );

    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::Done,
        "so the Conversation is where the press found it",
    );
}

/// And a Conversation that is not Done is not one to send back to a wrap-up,
/// whatever its pull request says about merging.
///
/// One that is wrapping up already has the watchers on it and needs no press;
/// one that has been closed since the pane was drawn is not one to start work
/// in.
#[tokio::test]
async fn only_a_done_conversation_is_sent_back_to_wrapping_up() {
    let (_dir, pool) = fresh_pool().await;

    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();
    record_merging(&pool, id, own, Merging::Conflicting)
        .await
        .unwrap();

    assert_eq!(
        resolve_conflicts(&pool, id).await.unwrap(),
        Resolving::NotDone,
        "a wrap-up is watching this already",
    );

    close_conversation(&pool, id).await.unwrap();

    assert_eq!(
        resolve_conflicts(&pool, id).await.unwrap(),
        Resolving::NotDone,
        "and the human is finished with this one",
    );

    assert_eq!(
        resolve_conflicts(&pool, 404).await.unwrap(),
        Resolving::NoSuchConversation,
    );
}
