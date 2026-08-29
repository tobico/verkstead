//! Registering Repos: what a registration records, what it refuses, that it is
//! still there after the server has been restarted, and what taking one away
//! does to all of that.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Lifecycle, Unregistering, load_repo, open_database, register_repo, registered_repos, set_state,
    start_conversation, unregister_repo,
};

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

/// Taking a Repo away takes it off every list that offers Repos for new work —
/// which is this read, the one the settings list, the New conversation menu and
/// the abandoned-roadmaps notice are all drawn from.
#[tokio::test]
async fn an_unregistered_repo_is_off_the_list() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing was registered at that path yet");

    assert_eq!(
        unregister_repo(&pool, repo.id).await.unwrap(),
        Unregistering::Unregistered
    );

    assert!(registered_repos(&pool).await.unwrap().is_empty());
}

/// And leaves the row where it is, because every Conversation ever started on it
/// names it by id: a Timeline goes on saying which repository its work was done
/// in, whatever the settings list is offering now.
#[tokio::test]
async fn an_unregistered_repo_still_resolves_by_id() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    // Something that is over, so the removal is not refused for it.
    let over = start_conversation(&pool, repo.id, "rate-limiting")
        .await
        .unwrap()
        .unwrap();
    set_state(&pool, over, Lifecycle::Done).await.unwrap();

    unregister_repo(&pool, repo.id).await.unwrap();

    assert_eq!(load_repo(&pool, repo.id).await.unwrap(), Some(repo));
}

/// Work still going on in a repository is the reason to keep it registered, so
/// the removal is refused while there is any — live being everything that is
/// neither Done nor Closed, which is the count the Repo's own pane shows.
#[tokio::test]
async fn a_repo_with_live_work_on_it_cannot_be_unregistered() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    let going = start_conversation(&pool, repo.id, "rate-limiting")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        unregister_repo(&pool, repo.id).await.unwrap(),
        Unregistering::InUse
    );
    assert_eq!(registered_repos(&pool).await.unwrap().len(), 1);

    // And once that Conversation is over, there is nothing left to refuse for.
    set_state(&pool, going, Lifecycle::Closed).await.unwrap();

    assert_eq!(
        unregister_repo(&pool, repo.id).await.unwrap(),
        Unregistering::Unregistered
    );
}

/// An id nothing was ever registered under, and one that has already been taken
/// away, are the same answer: neither names a Repo that is on the registry.
#[tokio::test]
async fn there_is_nothing_to_unregister_twice() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    unregister_repo(&pool, repo.id).await.unwrap();

    assert_eq!(
        unregister_repo(&pool, repo.id).await.unwrap(),
        Unregistering::NoSuchRepo
    );
    assert_eq!(
        unregister_repo(&pool, 404).await.unwrap(),
        Unregistering::NoSuchRepo
    );
}

/// Registering a path a taken-away Repo still holds revives that row rather than
/// being refused as registered already — the same Repo, under the same id, with
/// whatever the repository is called now.
#[tokio::test]
async fn registering_an_unregistered_path_again_revives_it() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    unregister_repo(&pool, repo.id).await.unwrap();

    let again = register_repo(&pool, Path::new("/watched/verkstead"), "renamed", "trunk")
        .await
        .unwrap()
        .expect("a taken-away path is registered again rather than refused");

    assert_eq!(again.id, repo.id, "the same row, revived");
    assert_eq!(again.name, "renamed");
    assert_eq!(again.default_branch, "trunk");
    assert_eq!(registered_repos(&pool).await.unwrap(), vec![again]);
}
