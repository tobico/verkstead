//! The pull request a Conversation's work was carried to, and the move into
//! Wrapping that recording one is.
//!
//! Recording the PR is not a thing that happens beside the move — it *is* the
//! move, in one transaction. So what these ask is what a Conversation and its
//! Timeline say afterwards: the state, the PR Event, and the move under it.
//!
//! Two states get here, because two kinds of work end on a pull request: a
//! backlog worked to empty, from Implementing, and a roadmap, from Grilling. And
//! two doors beside them: a Draft holding a pull request somebody opened
//! elsewhere, and a Tinker in Follow-up whose ending sent for one.
//!
//! And a Conversation can arrive twice. A review that split its findings out
//! into a backlog sends the work back to be built, and its finish step wraps up
//! again on the pull request it already had.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    AdoptedPullRequest, Entering, Event, Finished, Landing, Lifecycle, Merging, PullRequest,
    Rebuilding, Resolving, Rollup, Standing, Taking, WAITED_ON, WaitingOn, Wrapping, check_rollup,
    close_conversation, conversation_on_pull_request, finish_wrap_up, hold_pull_request,
    implement_again, load_conversation, merges, merging, open_database, pick_direction,
    pull_request, pull_request_repo, pull_requests, record_another_pull_request,
    record_check_rollup, record_merging, record_pull_request, record_standing, register_repo,
    resolve_conflicts, rollups, save_brief, settle_wrap_up, stack, standing, start_conversation,
    start_grilling, start_tinkering, take_up, timeline, unfinished_pull_requests, wrap_up_settled,
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

/// And a **Tinker**, which is a Conversation the one start press put straight
/// into Follow-up — the fourth state a pull request can be recorded against.
///
/// Walked there by the press that makes one, for the reason [`grilling`] is
/// walked: a Conversation dropped into a state by hand is one nothing else in the
/// store agrees about.
async fn tinkering(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let id = start_conversation(pool, repo.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();
    start_tinkering(
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

/// The PR the finish step opened, as the host's `gh` read it back.
fn opened() -> PullRequest {
    PullRequest {
        number: 41,
        title: "Rate limiting".to_owned(),
        url: "https://github.com/tobico/verkstead/pull/41".to_owned(),
        head: Some("rate-limiting".to_owned()),
        base: None,
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
        .chain(opened.into_iter().flat_map(|(repo, opened)| {
            let (repo_id, number) = (repo.id, opened.number);

            [
                WaitingOn::Checks { repo_id, number },
                WaitingOn::Comments { repo_id, number },
                WaitingOn::Mergeable { repo_id, number },
            ]
        }))
        .collect()
}

/// Which Timeline Event each of a Conversation's pull requests is, in the order
/// they were recorded.
///
/// What both the rollup map and the merge map are keyed by, that being what a card
/// has to hand — see [`verkstead_store::rollups`].
async fn pull_request_events(pool: &SqlitePool, id: i64) -> Vec<i64> {
    timeline(pool, id)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| matches!(event.event, Event::PullRequest(_)))
        .map(|event| event.id)
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

/// An ordinary Draft is not one to move into Wrapping, whatever recorded a pull
/// request against it: what makes a Draft wrappable is the pull request it is
/// holding, and this one is holding none.
#[tokio::test]
async fn a_draft_holding_no_pull_request_is_not_moved_on_by_one() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let id = start_conversation(&pool, repo.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    assert_eq!(
        record_pull_request(&pool, id, repo.id, &opened())
            .await
            .unwrap(),
        Wrapping::NothingToWrap,
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Draft);
}

/// And one that *is* holding a pull request is: the take-up has put it on that
/// pull request's branch, and this record is the move — the same move the finish
/// step makes, reached by the other door.
#[tokio::test]
async fn a_draft_holding_a_pull_request_is_moved_on_by_recording_it() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let id = start_conversation(&pool, repo.id, "verkstead-1")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    hold_pull_request(
        &pool,
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

    assert_eq!(
        take_up(
            &pool,
            id,
            "rate-limiting",
            "c0ffee",
            Path::new("/state/worktrees/rate-limiting"),
            &[],
            Entering {
                landing: Landing::Drafting,
                settled: &[],
            },
        )
        .await
        .unwrap(),
        Taking::Recorded,
    );

    assert_eq!(
        record_pull_request(&pool, id, repo.id, &opened())
            .await
            .unwrap(),
        Wrapping::Started,
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Wrapping);
    assert_eq!(conversation.branch, "rate-limiting");
    assert_eq!(conversation.base_commit.as_deref(), Some("c0ffee"));

    assert_eq!(
        events(&pool, id)
            .await
            .into_iter()
            .filter_map(|event| match event {
                Event::Moved(state) => Some(state),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [Lifecycle::Wrapping],
        "one move, from the Draft it was straight into the wrap-up",
    );
}

/// And a take-up may enter Wrapping with things already settled, which is a
/// wrap-up narrowed before it looks.
///
/// **Fix Merge Issues** is what asks for it: the review and the pull request's
/// comments are settled as the Conversation lands, so what its wrap-up waits on is
/// whether the checks are green and whether GitHub will merge it. They are written
/// in this transaction rather than after it — over a bare branch this *is* the
/// move, and over a pull request it is the transaction before the one that makes
/// it — so there is no moment in which a sweep or a restart could find the
/// Conversation wrapping up and not narrowed.
///
/// Asked over the bare branch here, that being the door where the move and the
/// settles are the one write: the Conversation is Wrapping when this returns and
/// both are already down. What the store does with what it is handed is the whole
/// of what this asks — which pull request the comments are settled against is the
/// caller's, and a settlement names one by the Repo and the number together now
/// that a repository can hold a whole stack of them.
#[tokio::test]
async fn a_take_up_can_enter_wrapping_with_the_review_and_the_comments_settled() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let id = start_conversation(&pool, repo.id, "verkstead-1")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    assert_eq!(
        take_up(
            &pool,
            id,
            "rate-limiting",
            "c0ffee",
            Path::new("/state/worktrees/rate-limiting"),
            &[],
            Entering {
                landing: Landing::Wrapping,
                settled: &[
                    WaitingOn::Review,
                    WaitingOn::Comments {
                        repo_id: repo.id,
                        number: 41
                    }
                ],
            },
        )
        .await
        .unwrap(),
        Taking::Recorded,
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.state, Lifecycle::Wrapping);

    let settled = wrap_up_settled(&pool, id).await.unwrap();

    assert!(
        settled.contains(&WaitingOn::Review),
        "the review is settled as the Conversation lands: {settled:?}",
    );
    assert!(
        settled.contains(&WaitingOn::Comments {
            repo_id: repo.id,
            number: 41
        }),
        "and so is what is said on the pull request it was handed: {settled:?}",
    );
    assert!(
        !settled.contains(&WaitingOn::Checks {
            repo_id: repo.id,
            number: 41
        }) && !settled.contains(&WaitingOn::Mergeable {
            repo_id: repo.id,
            number: 41
        }),
        "and nothing else is: those two are what a narrowed wrap-up waits on: {settled:?}",
    );
}

/// And a Conversation in Follow-up is: a Tinker whose rounds committed on a
/// branch that is on no pull request, whose ending sent a session to open one.
///
/// The fourth door, and the one that saves a second wrap-up entry being written
/// beside the ending — what the ending does is send for the pull request, and
/// recording one is the move, exactly as it is for every other ending. A
/// follow-up steered into never comes through here: it is on a pull request
/// already, and its ending lands it back in the wrap-up it was opened over.
#[tokio::test]
async fn a_follow_up_is_moved_on_by_the_pull_request_its_ending_sent_for() {
    let (_dir, pool) = fresh_pool().await;
    let id = tinkering(&pool).await;

    assert_eq!(
        load_conversation(&pool, id).await.unwrap().unwrap().state,
        Lifecycle::FollowUp,
        "the press landed it there rather than in a grilling",
    );

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
        head: Some("rate-limiting".to_owned()),
        base: None,
        repo: None,
    }
}

/// A pull request already on the record reuses the row it has. Which is what makes
/// a discovery that runs twice do nothing the second time — and what a second wrap
/// lands on.
#[tokio::test]
async fn a_pull_request_already_on_the_record_keeps_the_row_it_has() {
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

    assert_eq!(
        pull_request_events(&pool, id).await.len(),
        2,
        "one row per pull request, and no more",
    );
}

/// And another number in the same repository stands beside it, which is what a
/// stack is: a chain of pull requests each based on the one below, all in the one
/// place.
///
/// Every one of them reads back with its own checks, its own merge reading and its
/// own standing, because every one of those is keyed by the pull request rather
/// than by the repository it is in.
#[tokio::test]
async fn a_stack_is_several_pull_requests_of_one_repository() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    let own = own(&pool, id).await;

    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();

    for above in [
        stacked(42, "rate-limiting-2"),
        stacked(43, "rate-limiting-3"),
    ] {
        assert!(
            record_another_pull_request(&pool, id, own, &above)
                .await
                .unwrap(),
            "the chain above the named pull request is recorded in the same repository",
        );
    }

    assert_eq!(
        pull_requests(&pool, id)
            .await
            .unwrap()
            .into_iter()
            .map(|(repo, opened)| (repo.id, opened.number, opened.head))
            .collect::<Vec<_>>(),
        [
            (own, 41, Some("rate-limiting".to_owned())),
            (own, 42, Some("rate-limiting-2".to_owned())),
            (own, 43, Some("rate-limiting-3".to_owned())),
        ],
        "all three read back in the order they were recorded, each on its own branch",
    );

    // A reading of GitHub apiece, and each of the three different from the two
    // beside it: a stack whose bottom branch has stopped merging is exactly that
    // shape.
    for (number, rollup, merges, stands) in [
        (41, Rollup::Failed, Merging::Conflicting, Standing::Open),
        (42, Rollup::Running, Merging::Cleanly, Standing::Open),
        (43, Rollup::Passed, Merging::Cleanly, Standing::Merged),
    ] {
        record_check_rollup(&pool, id, own, number, rollup)
            .await
            .unwrap();
        record_merging(&pool, id, own, number, merges)
            .await
            .unwrap();
        record_standing(&pool, id, own, number, stands)
            .await
            .unwrap();
    }

    for (number, rollup, merges, stands) in [
        (41, Rollup::Failed, Merging::Conflicting, Standing::Open),
        (42, Rollup::Running, Merging::Cleanly, Standing::Open),
        (43, Rollup::Passed, Merging::Cleanly, Standing::Merged),
    ] {
        assert_eq!(
            check_rollup(&pool, id, own, number).await.unwrap(),
            Some(rollup),
            "pull request {number} keeps its own suite",
        );
        assert_eq!(
            merging(&pool, id, own, number).await.unwrap(),
            Some(merges),
            "and its own merge reading",
        );
        assert_eq!(
            standing(&pool, id, own, number).await.unwrap(),
            Some(stands),
            "and its own standing",
        );
    }
}

/// One pull request of the chain above the named one: the same repository, a
/// number of its own, and the branch its work is on.
fn stacked(number: i64, head: &str) -> PullRequest {
    PullRequest {
        number,
        title: "Rate limiting".to_owned(),
        url: format!("https://github.com/tobico/verkstead/pull/{number}"),
        head: Some(head.to_owned()),
        base: None,
        repo: None,
    }
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
///
/// Per pull request rather than per Conversation, which is what a suite is about:
/// a Conversation with a read-write companion has two of them, each watched on its
/// own interval, and one row between them would be each watcher reading the
/// other's suite.
#[tokio::test]
async fn how_the_checks_are_is_written_down_and_read_back() {
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
        check_rollup(&pool, id, own, 41).await.unwrap(),
        None,
        "nothing has asked GitHub yet, which is not the same as green",
    );

    assert!(
        record_check_rollup(&pool, id, own, 41, Rollup::Running)
            .await
            .unwrap(),
        "the first poll is news",
    );
    assert_eq!(
        check_rollup(&pool, id, own, 41).await.unwrap(),
        Some(Rollup::Running)
    );

    assert!(
        !record_check_rollup(&pool, id, own, 41, Rollup::Running)
            .await
            .unwrap(),
        "and a suite still running half an hour later is the same thing said again",
    );

    assert!(
        record_check_rollup(&pool, id, own, 41, Rollup::Failed)
            .await
            .unwrap(),
        "a check going red is news",
    );
    assert_eq!(
        check_rollup(&pool, id, own, 41).await.unwrap(),
        Some(Rollup::Failed)
    );

    // And the companion's suite, which is a suite of its own: it went green while
    // the work's own was red, and neither reading is the other's.
    record_check_rollup(&pool, id, beside, 7, Rollup::Passed)
        .await
        .unwrap();

    assert_eq!(
        check_rollup(&pool, id, own, 41).await.unwrap(),
        Some(Rollup::Failed),
        "the companion going green is nothing about the work's own suite",
    );
    assert_eq!(
        check_rollup(&pool, id, beside, 7).await.unwrap(),
        Some(Rollup::Passed),
    );

    assert!(
        record_check_rollup(&pool, id, own, 41, Rollup::Passed)
            .await
            .unwrap(),
        "and the fix session's push going green is news",
    );
    assert_eq!(
        check_rollup(&pool, id, own, 41).await.unwrap(),
        Some(Rollup::Passed)
    );
}

/// And every one of them together, by the Timeline Event each pull request is,
/// which is what the Conversation view draws its icons off.
///
/// Keyed by the Event for the merge map's reason — a pull request's card is drawn
/// pinned above the record and at the moment it opened, and both copies know only
/// which Event they are.
///
/// Every pull request rather than the Conversation's own, which is the whole of
/// what this slice changed: a companion's card drew no icon at all while the
/// rollup was the Conversation's, and draws its own now.
#[tokio::test]
async fn every_pull_requests_rollup_is_read_back_by_the_event_it_is() {
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
        rollups(&pool, id).await.unwrap().is_empty(),
        "nothing has asked GitHub about either of them, so there is nothing to draw",
    );

    record_check_rollup(&pool, id, own, 41, Rollup::Failed)
        .await
        .unwrap();
    record_check_rollup(&pool, id, beside, 7, Rollup::Passed)
        .await
        .unwrap();

    let events = pull_request_events(&pool, id).await;

    assert_eq!(
        rollups(&pool, id).await.unwrap(),
        std::collections::HashMap::from(
            [(events[0], Rollup::Failed), (events[1], Rollup::Passed),]
        ),
        "each Event carries the suite of its own pull request, where the second \
         written used to stand for both",
    );
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
    let own = own(&pool, id).await;
    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();
    record_check_rollup(&pool, id, own, 41, Rollup::Passed)
        .await
        .unwrap();
    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    assert_eq!(
        check_rollup(&pool, id, own, 41).await.unwrap(),
        Some(Rollup::Passed)
    );
}

/// And whether it merges, which is the other reading of GitHub written down here —
/// kept per pull request, as the rollup above it is, because a conflict is a fact
/// about one branch and its base.
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
        merging(&pool, id, own, 41).await.unwrap(),
        None,
        "nothing has asked GitHub yet, which is not the same as merging cleanly",
    );

    assert!(
        record_merging(&pool, id, own, 41, Merging::Conflicting)
            .await
            .unwrap(),
        "the first poll is news",
    );
    record_merging(&pool, id, beside, 7, Merging::Cleanly)
        .await
        .unwrap();

    assert_eq!(
        merging(&pool, id, own, 41).await.unwrap(),
        Some(Merging::Conflicting),
    );
    assert_eq!(
        merging(&pool, id, beside, 7).await.unwrap(),
        Some(Merging::Cleanly),
        "the companion's own base has not moved, and its pull request says so",
    );

    assert!(
        !record_merging(&pool, id, own, 41, Merging::Conflicting)
            .await
            .unwrap(),
        "and a conflict still standing on the next poll is the same thing said again",
    );

    // And the conflict resolved: written over rather than added to, a conflict
    // that has been dealt with not being a conflict.
    assert!(
        record_merging(&pool, id, own, 41, Merging::Cleanly)
            .await
            .unwrap(),
        "a resolution landing is news, which is what takes the mark off the card",
    );

    assert_eq!(
        merging(&pool, id, own, 41).await.unwrap(),
        Some(Merging::Cleanly)
    );
}

/// And every one of them together, by the Timeline Event each pull request is,
/// which is what the Conversation view draws its marks off.
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

    record_merging(&pool, id, own, 41, Merging::Conflicting)
        .await
        .unwrap();

    // Which Event each pull request is, in the order they were recorded — the
    // Conversation's own first, then the companion's.
    let events = pull_request_events(&pool, id).await;

    assert_eq!(
        merges(&pool, id).await.unwrap(),
        std::collections::HashMap::from([(events[0], Merging::Conflicting)]),
        "the one that was asked about is in it, and the one that was not is absent",
    );

    record_merging(&pool, id, beside, 7, Merging::Cleanly)
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
    record_merging(&pool, id, own, 41, Merging::Conflicting)
        .await
        .unwrap();
    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    assert_eq!(
        merging(&pool, id, own, 41).await.unwrap(),
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
        standing(&pool, id, own, 41).await.unwrap(),
        None,
        "nothing has asked GitHub yet, which is not the same as being open",
    );

    record_standing(&pool, id, own, 41, Standing::Open)
        .await
        .unwrap();
    record_standing(&pool, id, beside, 7, Standing::Merged)
        .await
        .unwrap();

    assert_eq!(
        standing(&pool, id, own, 41).await.unwrap(),
        Some(Standing::Open)
    );
    assert_eq!(
        standing(&pool, id, beside, 7).await.unwrap(),
        Some(Standing::Merged),
        "the companion's half has landed and the work's own has not, which is two \
         repositories being two repositories",
    );

    // And the merge beside it is untouched by any of that: a pull request that
    // has been merged merged cleanly.
    record_merging(&pool, id, own, 41, Merging::Conflicting)
        .await
        .unwrap();

    assert_eq!(
        standing(&pool, id, own, 41).await.unwrap(),
        Some(Standing::Open)
    );
    assert_eq!(
        merging(&pool, id, own, 41).await.unwrap(),
        Some(Merging::Conflicting),
    );

    // Written over rather than added to, as every reading of GitHub here is.
    record_standing(&pool, id, own, 41, Standing::Closed)
        .await
        .unwrap();

    assert_eq!(
        standing(&pool, id, own, 41).await.unwrap(),
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
    record_standing(&pool, wrapping, own, 41, Standing::Open)
        .await
        .unwrap();

    assert_eq!(unfinished_pull_requests(&pool).await.unwrap().len(), 1);

    // And one that says somebody has merged it takes it off for good.
    record_standing(&pool, wrapping, own, 41, Standing::Merged)
        .await
        .unwrap();

    assert_eq!(
        unfinished_pull_requests(&pool).await.unwrap(),
        Vec::new(),
        "a merged pull request is a question with a final answer",
    );
}

/// And a stack is walked one pull request at a time: three in one repository are
/// three questions, and the one somebody has merged leaves the walk without taking
/// the two beside it with it.
#[tokio::test]
async fn a_stack_leaves_the_sweep_one_pull_request_at_a_time() {
    let (_dir, pool) = fresh_pool().await;

    let id = implementing(&pool).await;
    let own = own(&pool, id).await;

    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();

    for above in [
        stacked(42, "rate-limiting-2"),
        stacked(43, "rate-limiting-3"),
    ] {
        record_another_pull_request(&pool, id, own, &above)
            .await
            .unwrap();
    }

    for waiting_on in waiting_on(&pool, id).await {
        settle_wrap_up(&pool, id, waiting_on).await.unwrap();
    }
    assert_eq!(finish_wrap_up(&pool, id).await.unwrap(), Finished::Done);

    assert_eq!(
        unfinished_pull_requests(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|unfinished| unfinished.number)
            .collect::<Vec<_>>(),
        [41, 42, 43],
        "every pull request of the stack is asked about, in the order they were \
         recorded",
    );

    // The human merges the bottom of the stack, which is the one that can land
    // first.
    record_standing(&pool, id, own, 41, Standing::Merged)
        .await
        .unwrap();

    assert_eq!(
        unfinished_pull_requests(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|unfinished| unfinished.number)
            .collect::<Vec<_>>(),
        [42, 43],
        "and the two above it are still questions with no final answer",
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
    record_standing(&pool, id, own, 41, Standing::Merged)
        .await
        .unwrap();
    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    assert_eq!(
        standing(&pool, id, own, 41).await.unwrap(),
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
    record_merging(&pool, id, own, 41, Merging::Conflicting)
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
                Event::Steer(target, ..) => Some(format!("steered into {target:?}")),
                Event::ResolveConflicts => Some("pressed resolve".to_owned()),
                Event::Moved(state) => Some(format!("moved to {state:?}")),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [
            "moved to Grilling",
            "moved to Wrapping",
            "moved to Done",
            // The press, in a steer's own shape and a kind of its own: somebody
            // decided this, and the move under it is what came of it. Its own
            // kind because a steer into Wrapping carrying nothing written would
            // otherwise be the same row, and the two are different acts — that
            // one reads the branch again and this one leaves the review alone.
            "pressed resolve",
            "moved to Wrapping",
        ]
        .map(str::to_owned),
        "the human's line lands above the machine's move, and says which press \
         it was",
    );

    let settled = wrap_up_settled(&pool, id).await.unwrap();

    assert!(
        !settled.contains(&WaitingOn::Mergeable {
            repo_id: own,
            number: 41
        }),
        "the conflict the press was made over is something the wrap-up waits on \
         again: {settled:?}",
    );
    assert!(
        settled.contains(&WaitingOn::Review),
        "and the review it was carried to Done on stands, so nothing reads the \
         branch a second time: {settled:?}",
    );
    assert!(
        settled.contains(&WaitingOn::Checks {
            repo_id: own,
            number: 41
        }) && settled.contains(&WaitingOn::Comments {
            repo_id: own,
            number: 41
        }),
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

    record_merging(&pool, id, own, 41, Merging::Cleanly)
        .await
        .unwrap();
    record_merging(&pool, id, beside, 7, Merging::Conflicting)
        .await
        .unwrap();

    assert_eq!(
        resolve_conflicts(&pool, id).await.unwrap(),
        Resolving::Wrapping,
    );

    let settled = wrap_up_settled(&pool, id).await.unwrap();

    assert!(
        !settled.contains(&WaitingOn::Mergeable {
            repo_id: beside,
            number: 7
        }),
        "the companion's branch is the one that stopped merging: {settled:?}",
    );
    assert!(
        settled.contains(&WaitingOn::Mergeable {
            repo_id: own,
            number: 41
        }),
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

    record_merging(&pool, id, own, 41, Merging::Cleanly)
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
    record_merging(&pool, id, own, 41, Merging::Conflicting)
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

/// One link of a chain: a number, the branch its work is on, and the branch it
/// merges into — which together are what says which of a stack sits on which.
fn link(number: i64, head: &str, base: &str) -> PullRequest {
    PullRequest {
        number,
        title: format!("Stage 0{number}"),
        url: format!("https://github.com/tobico/verkstead/pull/{number}"),
        head: Some(head.to_owned()),
        base: Some(base.to_owned()),
        repo: None,
    }
}

/// A stack reads back from the bottom, whatever order its pull requests were
/// recorded in.
///
/// Which is the whole reason the base is on the row: what a Conversation was
/// pointed at is recorded first — that is the move — and the chain it turned out
/// to be a link of is recorded around it. So the order they arrived in is the
/// order the human named them in, and the order they *sit* in is the one the
/// branches say.
#[tokio::test]
async fn a_stack_reads_back_from_the_bottom_whatever_order_it_was_recorded_in() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    let own = own(&pool, id).await;

    // The middle of the chain, which is the one the Conversation is on.
    record_pull_request(&pool, id, own, &link(42, "stage-02", "stage-01"))
        .await
        .unwrap();

    // And the two around it, recorded top first so that the order they arrived
    // in is nothing like the order they sit in.
    for beside in [
        link(43, "stage-03", "stage-02"),
        link(41, "stage-01", "main"),
    ] {
        assert!(
            record_another_pull_request(&pool, id, own, &beside)
                .await
                .unwrap()
        );
    }

    assert_eq!(
        stack(&pool, id, own)
            .await
            .unwrap()
            .into_iter()
            .map(|opened| opened.number)
            .collect::<Vec<_>>(),
        [41, 42, 43],
        "from the bottom: the one nothing else is based on, then each one up",
    );

    assert_eq!(
        pull_requests(&pool, id)
            .await
            .unwrap()
            .into_iter()
            .map(|(_, opened)| opened.number)
            .collect::<Vec<_>>(),
        [42, 43, 41],
        "and the record still says which one the Conversation was pointed at first",
    );
}

/// A lone pull request is a chain of one, and a repository the Conversation has
/// nothing in is a chain of none.
#[tokio::test]
async fn a_lone_pull_request_is_a_stack_of_one() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    let own = own(&pool, id).await;
    let beside = companion(&pool).await;

    record_pull_request(&pool, id, own, &opened())
        .await
        .unwrap();

    assert_eq!(
        stack(&pool, id, own)
            .await
            .unwrap()
            .into_iter()
            .map(|opened| opened.number)
            .collect::<Vec<_>>(),
        [41],
    );
    assert!(
        stack(&pool, id, beside).await.unwrap().is_empty(),
        "and a companion with nothing recorded in it has no chain at all",
    );
}

/// Another Conversation of the same Repo, carried to Implementing the way
/// [`implementing`] carries the first: a stack in this workbench is a pull
/// request per Conversation, so the neighbours have Conversations of their own.
async fn beside_it_in(pool: &SqlitePool, repo: i64, branch: &str) -> i64 {
    let id = start_conversation(pool, repo, branch)
        .await
        .unwrap()
        .unwrap();

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();
    start_grilling(
        pool,
        id,
        "c0ffee",
        &std::path::PathBuf::from(format!("/state/worktrees/{branch}")),
        &[],
    )
    .await
    .unwrap();

    pick_direction(pool, id, verkstead_schema::Direction::Inline)
        .await
        .unwrap();

    id
}

/// Rows that do not make one chain come back in the order they were recorded,
/// which is the order every reader had before there were stacks.
///
/// Two of them, because there are two ways to not be a chain: rows that say
/// nothing about what they sit on, and rows that say two things sit on the same
/// branch.
#[tokio::test]
async fn pull_requests_that_are_not_one_chain_read_in_the_order_they_were_recorded() {
    for beside in [
        vec![
            stacked(42, "rate-limiting-2"),
            stacked(43, "rate-limiting-3"),
        ],
        vec![
            link(42, "stage-02", "stage-01"),
            link(43, "stage-02-again", "stage-01"),
        ],
    ] {
        let (_dir, pool) = fresh_pool().await;
        let id = implementing(&pool).await;
        let own = own(&pool, id).await;

        record_pull_request(&pool, id, own, &link(41, "stage-01", "main"))
            .await
            .unwrap();

        for one in beside {
            assert!(
                record_another_pull_request(&pool, id, own, &one)
                    .await
                    .unwrap()
            );
        }

        assert_eq!(
            stack(&pool, id, own)
                .await
                .unwrap()
                .into_iter()
                .map(|opened| opened.number)
                .collect::<Vec<_>>(),
            [41, 42, 43],
        );
    }
}

/// The pull request a press is refused over is the one a Conversation's work is
/// *on*, rather than every row recorded beside it.
///
/// Which is what recording a stack made worth asking. A wrap-up over a chain of
/// three has three rows, and two of them are pull requests it is watching rather
/// than holding — so a second Conversation pointed at one of those is pointed at
/// work nobody has taken up, and the Conversation that *has* taken one up is
/// still the one a press is sent to.
#[tokio::test]
async fn the_conversation_a_pull_request_leads_to_is_the_one_whose_work_is_on_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = implementing(&pool).await;
    let own = own(&pool, id).await;

    record_pull_request(&pool, id, own, &link(42, "stage-02", "stage-01"))
        .await
        .unwrap();

    for beside in [
        link(41, "stage-01", "main"),
        link(43, "stage-03", "stage-02"),
    ] {
        assert!(
            record_another_pull_request(&pool, id, own, &beside)
                .await
                .unwrap()
        );
    }

    assert_eq!(
        conversation_on_pull_request(&pool, own, 42).await.unwrap(),
        Some(id),
        "the one it was pointed at is the one it is on",
    );
    assert_eq!(
        conversation_on_pull_request(&pool, own, 41).await.unwrap(),
        None,
        "and the neighbours are watched rather than held",
    );
    assert_eq!(
        conversation_on_pull_request(&pool, own, 43).await.unwrap(),
        None,
    );

    // And the Conversation that is on one of them is what a press is sent to,
    // which is the refusal in full: its own row is the first in its repository,
    // whatever anybody else recorded beside it.
    let neighbour = beside_it_in(&pool, own, "stage-01").await;

    record_pull_request(&pool, neighbour, own, &link(41, "stage-01", "main"))
        .await
        .unwrap();

    assert_eq!(
        conversation_on_pull_request(&pool, own, 41).await.unwrap(),
        Some(neighbour),
    );

    // A companion's pull request is its repository's first and stays a refusal:
    // a Conversation ends on one per repository it committed in, and none of
    // them is a neighbour.
    let beside = companion(&pool).await;

    assert!(
        record_another_pull_request(&pool, id, beside, &beside_it())
            .await
            .unwrap()
    );
    assert_eq!(
        conversation_on_pull_request(&pool, beside, 7)
            .await
            .unwrap(),
        Some(id),
    );
}
