//! A stage away from home, and the tombstone its home keeps for it (ADR-0029,
//! *Home drives a roadmap across devices*).
//!
//! Two halves of one database's worth. The far end's: which Conversations are a
//! stage away from home at all, and what was last told about each. And home's:
//! a standing written onto the tombstone reads, in `stage_standings`, what it
//! read on the copy that reported it — which is the whole point of writing it
//! there rather than teaching the readers somewhere else to look. The link
//! between the two is the server's — see its `tests/suite/transfer.rs`.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_store::{
    Birth, Lifecycle, LiveStanding, RoadmapStage, StageStanding, Transferred, away_from_home,
    follow, live_standing, open_database, record_birth, register_repo, stage_standings,
    stages_away_from_home, start_conversation, start_stage, tell, told, transfer_away,
};

/// The device this database is on.
const HERE: &str = "aa00bb11cc22dd33ee44ff5566778899";

/// And the stages' home, where they are copies here.
const HOME: &str = "0011223344556677889900aabbccddee";

async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

async fn repo(pool: &SqlitePool) -> i64 {
    register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet")
        .id
}

/// Stage `label` of the `mvp` roadmap, started here.
async fn stage(pool: &SqlitePool, repo_id: i64, label: &str) -> i64 {
    let id = start_conversation(pool, repo_id, &format!("roadmaps/mvp/{label}"), HERE, None)
        .await
        .unwrap()
        .unwrap();

    start_stage(
        pool,
        id,
        "c0ffee",
        &PathBuf::from("/data/worktrees").join(format!("mvp-{label}")),
        None,
        RoadmapStage {
            roadmap: "mvp",
            label,
        },
        &[],
    )
    .await
    .unwrap();

    id
}

/// The same stage as a copy that came here from `HOME`, where it was born as
/// `born_as`.
async fn copy(pool: &SqlitePool, repo_id: i64, label: &str, born_as: i64) -> i64 {
    let id = stage(pool, repo_id, label).await;

    record_birth(
        pool,
        id,
        &Birth {
            device: HOME.to_owned(),
            id: born_as,
        },
    )
    .await
    .unwrap();

    id
}

fn standing(state: Lifecycle, done: bool, wrapped: bool, stopped: bool) -> LiveStanding {
    LiveStanding {
        state,
        done,
        wrapped,
        stopped,
    }
}

/// A copy of a stage born elsewhere is away from home and says where home is;
/// a stage born here is not, and neither is a tombstone — whose work is
/// somewhere else — nor a Conversation that is no stage.
#[tokio::test]
async fn only_a_live_stage_born_elsewhere_is_away_from_home() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool).await;

    let away = copy(&pool, repo_id, "01", 41).await;
    let born_here = stage(&pool, repo_id, "02").await;

    let handed_on = copy(&pool, repo_id, "03", 43).await;
    transfer_away(
        &pool,
        handed_on,
        &Transferred {
            device: "ffeeddccbbaa00998877665544332211".to_owned(),
            id: 9,
        },
    )
    .await
    .unwrap();

    let no_stage = start_conversation(&pool, repo_id, "rate-limiting", HERE, None)
        .await
        .unwrap()
        .unwrap();
    record_birth(
        &pool,
        no_stage,
        &Birth {
            device: HOME.to_owned(),
            id: 44,
        },
    )
    .await
    .unwrap();

    assert_eq!(
        away_from_home(&pool, away, HERE).await.unwrap(),
        Some(Birth {
            device: HOME.to_owned(),
            id: 41,
        }),
    );

    for (id, why) in [
        (born_here, "a stage born here is at home"),
        (handed_on, "a tombstone holds no work to report on"),
        (
            no_stage,
            "and a Conversation that is no stage has no roadmap to tell",
        ),
    ] {
        assert_eq!(
            away_from_home(&pool, id, HERE).await.unwrap(),
            None,
            "{why}"
        );
    }

    assert_eq!(
        stages_away_from_home(&pool, HERE).await.unwrap(),
        [away],
        "the sweep walks the one stage away from home",
    );
}

/// What the far end last told home is remembered, so the same standing is not
/// owed twice — and a copy that has told nothing has nothing remembered.
#[tokio::test]
async fn what_home_was_told_is_remembered() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool).await;
    let away = copy(&pool, repo_id, "01", 41).await;

    let now = live_standing(&pool, away).await.unwrap().unwrap();

    assert_eq!(now, standing(Lifecycle::Implementing, false, false, false));
    assert_eq!(told(&pool, away).await.unwrap(), None);

    tell(&pool, away, &now).await.unwrap();
    assert_eq!(told(&pool, away).await.unwrap(), Some(now));

    let wrapping = standing(Lifecycle::Wrapping, false, true, false);
    tell(&pool, away, &wrapping).await.unwrap();
    assert_eq!(told(&pool, away).await.unwrap(), Some(wrapping));
}

/// **A stage closed on the far device reads abandoned at home**, and one that
/// wrapped up and closed reads settled — the tombstone read as its live copy is.
#[tokio::test]
async fn a_tombstone_reads_the_standing_its_live_copy_reported() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool).await;

    let abandoned = stage(&pool, repo_id, "01").await;
    let wrapped_then_closed = stage(&pool, repo_id, "02").await;

    assert_eq!(
        stage_standings(&pool, repo_id)
            .await
            .unwrap()
            .of("mvp", "01"),
        Some(StageStanding::InFlight),
    );

    follow(
        &pool,
        abandoned,
        &standing(Lifecycle::Closed, false, false, false),
    )
    .await
    .unwrap();

    follow(
        &pool,
        wrapped_then_closed,
        &standing(Lifecycle::Closed, false, true, false),
    )
    .await
    .unwrap();

    let standings = stage_standings(&pool, repo_id).await.unwrap();

    assert_eq!(standings.of("mvp", "01"), Some(StageStanding::Abandoned));
    assert_eq!(standings.of("mvp", "02"), Some(StageStanding::Settled));
}

/// The report that moves a tombstone into Done is the one that settles it — the
/// moment home carries the roadmap on — and the same report again settles
/// nothing, which is what lets the far end resend freely.
#[tokio::test]
async fn the_move_into_done_settles_once() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool).await;
    let tombstone = stage(&pool, repo_id, "01").await;

    let wrapping = standing(Lifecycle::Wrapping, false, true, false);
    let done = standing(Lifecycle::Done, true, true, false);

    assert!(!follow(&pool, tombstone, &wrapping).await.unwrap().settled);
    assert!(follow(&pool, tombstone, &done).await.unwrap().settled);
    assert!(
        !follow(&pool, tombstone, &done).await.unwrap().settled,
        "a resend of the same standing is not a second settle",
    );

    assert_eq!(
        stage_standings(&pool, repo_id)
            .await
            .unwrap()
            .of("mvp", "01"),
        Some(StageStanding::Settled),
    );

    assert_eq!(
        live_standing(&pool, tombstone).await.unwrap(),
        Some(done),
        "and the tombstone stands where the copy does",
    );
}

/// A stop on the far device is a stop on the tombstone, and the stop lifting
/// lifts it.
#[tokio::test]
async fn a_stop_and_its_lifting_follow_the_live_copy() {
    let (_dir, pool) = fresh_pool().await;
    let repo_id = repo(&pool).await;
    let tombstone = stage(&pool, repo_id, "01").await;

    follow(
        &pool,
        tombstone,
        &standing(Lifecycle::Implementing, false, false, true),
    )
    .await
    .unwrap();

    assert!(
        stage_standings(&pool, repo_id)
            .await
            .unwrap()
            .stopped("mvp", "01")
    );

    follow(
        &pool,
        tombstone,
        &standing(Lifecycle::Implementing, false, false, false),
    )
    .await
    .unwrap();

    assert!(
        !stage_standings(&pool, repo_id)
            .await
            .unwrap()
            .stopped("mvp", "01")
    );
}
