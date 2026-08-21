//! Registering Repos: what a registration records, what it refuses, and that it
//! is still there after the server has been restarted.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{open_database, register_repo, registered_repos};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

#[tokio::test]
async fn a_registered_repo_keeps_the_three_facts_it_was_given() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet");

    assert_eq!(repo.path, Path::new("/watched/verkstead"));
    assert_eq!(repo.name, "verkstead");
    assert_eq!(repo.default_branch, "main");

    assert_eq!(registered_repos(&pool).await.unwrap(), vec![repo]);
}

#[tokio::test]
async fn the_same_path_cannot_be_registered_twice() {
    let (_dir, pool) = fresh_pool().await;

    register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("the first registration should be taken");

    let again = register_repo(&pool, Path::new("/watched/verkstead"), "renamed", "trunk")
        .await
        .unwrap();

    assert!(again.is_none(), "the second registration should be refused");
    assert_eq!(registered_repos(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn registered_repos_come_back_by_name() {
    let (_dir, pool) = fresh_pool().await;

    for (path, name) in [
        ("/watched/verkstead", "verkstead"),
        ("/watched/askance", "askance"),
        ("/watched/other/askance", "askance"),
    ] {
        register_repo(&pool, Path::new(path), name, "main")
            .await
            .unwrap()
            .expect("each path is registered for the first time");
    }

    let listed: Vec<(String, String)> = registered_repos(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|repo| (repo.name, repo.path.display().to_string()))
        .collect();

    // By name, and by registration order where two directories share one.
    assert_eq!(
        listed,
        [
            ("askance".to_owned(), "/watched/askance".to_owned()),
            ("askance".to_owned(), "/watched/other/askance".to_owned()),
            ("verkstead".to_owned(), "/watched/verkstead".to_owned()),
        ]
    );
}

/// The point of registering in SQLite rather than in memory: the server is a
/// service that restarts, and a human who registered a repo last week should not
/// have to do it again.
#[tokio::test]
async fn registrations_survive_the_database_being_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let pool = open_database(&database).await.unwrap();
    register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("the registration should be taken");
    pool.close().await;

    let pool = open_database(&database).await.unwrap();
    let repos = registered_repos(&pool).await.unwrap();

    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].path, Path::new("/watched/verkstead"));
    assert_eq!(repos[0].default_branch, "main");
}

#[tokio::test]
async fn nothing_registered_means_nothing_listed() {
    let (_dir, pool) = fresh_pool().await;

    assert!(registered_repos(&pool).await.unwrap().is_empty());
}
