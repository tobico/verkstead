//! The one flag on this database that is about nothing on it: whether the human
//! is done with the banner pointing at Remote access.
//!
//! What is being asked of the store here is the whole reason the flag is not in
//! a browser's own storage. It is a dismissal made at a desk by somebody who is
//! about to pick up a phone, so what has to hold is that it outlives the
//! process the press landed in — a second device reading it back is the point of
//! the thing, and a restart is the strongest way to say so from here.

use verkstead_store::{dismiss_remote_banner, open_database, remote_banner_dismissed};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, sqlx::SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// Nobody has dismissed anything on a database nobody has used, which is where
/// the banner has something to say.
#[tokio::test]
async fn a_fresh_database_has_not_dismissed_the_banner() {
    let (_dir, pool) = fresh_pool().await;

    assert!(!remote_banner_dismissed(&pool).await.unwrap());
}

/// And the press is the end of it.
#[tokio::test]
async fn dismissing_it_ends_it() {
    let (_dir, pool) = fresh_pool().await;

    dismiss_remote_banner(&pool).await.unwrap();

    assert!(remote_banner_dismissed(&pool).await.unwrap());
}

/// Said twice, which is two devices whose loads crossed: the second press says
/// what the first one said rather than being refused.
#[tokio::test]
async fn saying_it_twice_says_the_same_thing() {
    let (_dir, pool) = fresh_pool().await;

    dismiss_remote_banner(&pool).await.unwrap();
    dismiss_remote_banner(&pool).await.unwrap();

    assert!(remote_banner_dismissed(&pool).await.unwrap());
}

/// And it outlives the process, which is the whole reason it is here rather
/// than on the device that made it.
#[tokio::test]
async fn the_dismissal_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("verkstead.db");

    {
        let pool = open_database(&path).await.unwrap();
        dismiss_remote_banner(&pool).await.unwrap();
        pool.close().await;
    }

    let pool = open_database(&path).await.unwrap();

    assert!(remote_banner_dismissed(&pool).await.unwrap());
}
