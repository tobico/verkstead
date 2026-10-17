//! Registering Repos: what a registration records, what it refuses, that it is
//! still there after the server has been restarted, and what taking one away
//! does to all of that.
//!
//! And what a registration is no longer told: how a merge conflict on its pull
//! requests is resolved. That was an override of the settings file, kept in a
//! table of its own, and an install that saved one comes up without it.
//!
//! And which Author each Repo is (ADR-0025): recorded by uuid at registering,
//! kept through an unregistering and a revive, and filled in once for the
//! Repos of an installation from before there was an Author list.

use std::path::Path;

use sqlx::SqlitePool;
use verkstead_store::{
    Adding, Lifecycle, Repointing, Unregistering, add_companion, load_repo, open_database,
    record_the_repo_authors, recorded_repos, register_repo, register_repo_as, registered_repo,
    registered_repos, repoint_repo, set_state, start_adoption, start_conversation, unregister_repo,
};

/// The device every Conversation started here is ranked by, named the way a
/// cluster names one (ADR-0020, *Ranks*).
const THIS_DEVICE: &str = "aa00bb11cc22dd33ee44ff5566778899";

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
/// which is this read, the one the settings list, the compose page's Repo
/// dropdown and the roadmaps there are to adopt are all drawn from.
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
    let over = start_conversation(&pool, repo.id, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .unwrap();
    set_state(&pool, over, Lifecycle::Done).await.unwrap();

    unregister_repo(&pool, repo.id).await.unwrap();

    assert_eq!(load_repo(&pool, repo.id).await.unwrap(), Some(repo));
}

/// Work still going on in a repository is the reason to keep it registered, so
/// the removal is refused while there is any — live being everything that is
/// neither Done nor Closed, which is `work_on_repo`'s own reading.
#[tokio::test]
async fn a_repo_with_live_work_on_it_cannot_be_unregistered() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    let going = start_conversation(&pool, repo.id, "rate-limiting", THIS_DEVICE)
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

/// Off the registry is not merely off the list: no new work goes into a Repo
/// that has been taken away, however the id got as far as the press. A page
/// that has not heard about the removal is the ordinary way that happens — one
/// device removes a Repo and another still has it in its compose page's Repo
/// dropdown.
#[tokio::test]
async fn nothing_new_is_started_in_an_unregistered_repo() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    unregister_repo(&pool, repo.id).await.unwrap();

    assert!(
        start_conversation(&pool, repo.id, "rate-limiting", THIS_DEVICE)
            .await
            .unwrap()
            .is_none(),
        "a Repo that was taken away is no Repo to start work in",
    );
    assert!(
        start_adoption(&pool, repo.id, "pane-paths", "mvp", THIS_DEVICE)
            .await
            .unwrap()
            .is_none(),
        "and adopting a roadmap in one is a start like any other",
    );

    // And registering it again is what makes it startable, the same press that
    // brought it back to the list.
    register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    assert!(
        start_conversation(&pool, repo.id, "rate-limiting", THIS_DEVICE)
            .await
            .unwrap()
            .is_some()
    );
}

/// And nothing already going is given one to work alongside: what a Conversation
/// may compose is what the human has put in the registry, which is the whole of
/// the trust boundary a companion sits behind.
#[tokio::test]
async fn an_unregistered_repo_is_no_companion() {
    let (_dir, pool) = fresh_pool().await;

    let own = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();
    let beside = register_repo(&pool, Path::new("/watched/askance"), "askance", "main")
        .await
        .unwrap()
        .unwrap();

    let conversation = start_conversation(&pool, own.id, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .unwrap();

    unregister_repo(&pool, beside.id).await.unwrap();

    assert_eq!(
        add_companion(&pool, conversation, beside.id).await.unwrap(),
        Adding::NoSuchRepo,
    );
}

/// Taking a Repo away does not take it off the list of Repos there *are*. The
/// directory is untouched by an unregistering, so git goes on holding whatever
/// registrations it held — and the sweep of orphaned worktrees has to prune it
/// like any other, whatever the settings list is offering now.
#[tokio::test]
async fn a_repo_taken_away_is_still_one_to_prune() {
    let (_dir, pool) = fresh_pool().await;

    let kept = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    let taken = register_repo(&pool, Path::new("/watched/askance"), "askance", "main")
        .await
        .unwrap()
        .unwrap();

    unregister_repo(&pool, taken.id).await.unwrap();

    assert_eq!(registered_repos(&pool).await.unwrap(), vec![kept.clone()]);
    assert_eq!(
        recorded_repos(&pool).await.unwrap(),
        vec![kept.path, taken.path],
    );
}

/// An install that had told one Repo to rebase comes up with nothing saying so.
///
/// The override is gone from every reader, and a table nothing reads is a
/// setting nobody can see they still have — so opening the database drops it,
/// beside the creates. Written here as the restart it is: a Verkstead that had
/// saved one, stopped, and been started again on the same file.
#[tokio::test]
async fn an_override_a_previous_verkstead_saved_is_dropped_on_open() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let pool = open_database(&database).await.unwrap();
    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("the registration should be taken");

    // What the Verkstead before this one wrote, in the shape it wrote it.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS repo_resolutions (
             repo_id  INTEGER PRIMARY KEY REFERENCES repos(id),
             strategy TEXT NOT NULL
         ) STRICT",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO repo_resolutions (repo_id, strategy) VALUES (?, 'rebase')")
        .bind(repo.id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    let held: Option<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind("repo_resolutions")
            .fetch_optional(&pool)
            .await
            .unwrap();

    assert_eq!(
        held, None,
        "the table the override was kept in is gone, so nothing can be answering out of it",
    );

    assert_eq!(
        registered_repos(&pool).await.unwrap().len(),
        1,
        "and the registration itself is untouched: the Repo is the same Repo",
    );
}

#[tokio::test]
async fn a_repo_registered_as_an_author_carries_its_uuid() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo_as(
        &pool,
        Path::new("/watched/verkstead"),
        "verkstead",
        "main",
        Some("work"),
    )
    .await
    .unwrap()
    .expect("nothing was registered at that path yet");

    assert_eq!(repo.author.as_deref(), Some("work"));
    assert_eq!(registered_repos(&pool).await.unwrap(), vec![repo.clone()]);
    assert_eq!(
        registered_repo(&pool, repo.id).await.unwrap(),
        Some(repo.clone())
    );
    assert_eq!(load_repo(&pool, repo.id).await.unwrap(), Some(repo));
}

#[tokio::test]
async fn a_repo_registered_as_nobody_has_no_author() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(repo.author, None);
    assert_eq!(
        load_repo(&pool, repo.id).await.unwrap().unwrap().author,
        None
    );
}

#[tokio::test]
async fn a_revived_repo_keeps_the_author_it_had() {
    let (_dir, pool) = fresh_pool().await;
    let path = Path::new("/watched/verkstead");

    let repo = register_repo_as(&pool, path, "verkstead", "main", Some("work"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        unregister_repo(&pool, repo.id).await.unwrap(),
        Unregistering::Unregistered
    );

    // Taken away, it is still on record as that Author's: the association is
    // history rather than something unregistering takes with it.
    assert_eq!(
        load_repo(&pool, repo.id)
            .await
            .unwrap()
            .unwrap()
            .author
            .as_deref(),
        Some("work")
    );

    let revived = register_repo_as(&pool, path, "verkstead", "main", Some("personal"))
        .await
        .unwrap()
        .expect("an unregistered path is revived rather than refused");

    assert_eq!(revived.id, repo.id);
    assert_eq!(revived.author.as_deref(), Some("work"));

    // And a revive that names nobody keeps it too.
    unregister_repo(&pool, repo.id).await.unwrap();
    let revived = register_repo(&pool, path, "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(revived.author.as_deref(), Some("work"));
}

#[tokio::test]
async fn the_repos_from_before_the_author_list_are_recorded_as_the_moved_author() {
    let (_dir, pool) = fresh_pool().await;

    let first = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();
    let second = register_repo(&pool, Path::new("/watched/askance"), "askance", "main")
        .await
        .unwrap()
        .unwrap();
    let recorded = register_repo_as(
        &pool,
        Path::new("/watched/other"),
        "other",
        "main",
        Some("kept"),
    )
    .await
    .unwrap()
    .unwrap();
    let gone = register_repo(&pool, Path::new("/watched/gone"), "gone", "main")
        .await
        .unwrap()
        .unwrap();
    unregister_repo(&pool, gone.id).await.unwrap();

    record_the_repo_authors(&pool, "moved").await.unwrap();
    // Twice, the way a start runs it, changing nothing the second time.
    record_the_repo_authors(&pool, "another").await.unwrap();

    let author = |id| {
        let pool = pool.clone();
        async move { load_repo(&pool, id).await.unwrap().unwrap().author }
    };

    assert_eq!(author(first.id).await.as_deref(), Some("moved"));
    assert_eq!(author(second.id).await.as_deref(), Some("moved"));
    assert_eq!(author(recorded.id).await.as_deref(), Some("kept"));
    assert_eq!(
        author(gone.id).await.as_deref(),
        Some("moved"),
        "an unregistered Repo is recorded too, so reviving it keeps an Author",
    );
}

#[tokio::test]
async fn the_author_is_still_there_after_the_database_is_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("verkstead.db");

    let id = {
        let pool = open_database(&path).await.unwrap();
        register_repo_as(
            &pool,
            Path::new("/watched/verkstead"),
            "verkstead",
            "main",
            Some("work"),
        )
        .await
        .unwrap()
        .unwrap()
        .id
    };

    let pool = open_database(&path).await.unwrap();

    assert_eq!(
        load_repo(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .author
            .as_deref(),
        Some("work")
    );
}

/// A Repo with no live work in it is pointed at another Author, replacing the
/// one it had — and every read of it says so after.
#[tokio::test]
async fn an_idle_repo_is_repointed() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo_as(
        &pool,
        Path::new("/watched/verkstead"),
        "verkstead",
        "main",
        Some("work"),
    )
    .await
    .unwrap()
    .unwrap();

    let going = start_conversation(&pool, repo.id, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .unwrap();
    set_state(&pool, going, Lifecycle::Done).await.unwrap();

    let Repointing::Repointed(repointed) = repoint_repo(&pool, repo.id, "home").await.unwrap()
    else {
        panic!("the repoint was refused");
    };

    assert_eq!(repointed.author.as_deref(), Some("home"));
    assert_eq!(load_repo(&pool, repo.id).await.unwrap(), Some(repointed));
}

/// Live work on the Repo refuses it, naming each branch, and live work holding
/// it as a Companion Repo refuses it too — named apart, the two being different
/// sentences. What is over refuses nothing.
#[tokio::test]
async fn a_repo_in_live_work_is_not_repointed() {
    let (_dir, pool) = fresh_pool().await;

    let own = register_repo_as(
        &pool,
        Path::new("/watched/verkstead"),
        "verkstead",
        "main",
        Some("work"),
    )
    .await
    .unwrap()
    .unwrap();
    let beside = register_repo_as(
        &pool,
        Path::new("/watched/askance"),
        "askance",
        "main",
        Some("work"),
    )
    .await
    .unwrap()
    .unwrap();

    let on = start_conversation(&pool, own.id, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .unwrap();
    let finished = start_conversation(&pool, own.id, "old-work", THIS_DEVICE)
        .await
        .unwrap()
        .unwrap();
    set_state(&pool, finished, Lifecycle::Closed).await.unwrap();

    assert_eq!(
        add_companion(&pool, on, beside.id).await.unwrap(),
        Adding::Added,
    );
    set_state(&pool, on, Lifecycle::Grilling).await.unwrap();

    assert_eq!(
        repoint_repo(&pool, own.id, "home").await.unwrap(),
        Repointing::InUse {
            on: vec!["rate-limiting".to_owned()],
            beside: vec![],
        },
    );
    assert_eq!(
        repoint_repo(&pool, beside.id, "home").await.unwrap(),
        Repointing::InUse {
            on: vec![],
            beside: vec!["rate-limiting".to_owned()],
        },
    );
    assert_eq!(
        load_repo(&pool, beside.id)
            .await
            .unwrap()
            .unwrap()
            .author
            .as_deref(),
        Some("work"),
        "nothing was recorded",
    );

    set_state(&pool, on, Lifecycle::Done).await.unwrap();

    assert!(matches!(
        repoint_repo(&pool, beside.id, "home").await.unwrap(),
        Repointing::Repointed(_),
    ));
}

/// A draft with nowhere to work yet holds nothing up: it has committed nothing
/// as anybody, and its cut asks every companion's Author again. Once it has a
/// Worktree it is live work like any other.
#[tokio::test]
async fn a_draft_holds_a_repoint_only_once_it_has_a_worktree() {
    let (_dir, pool) = fresh_pool().await;

    let own = register_repo_as(
        &pool,
        Path::new("/watched/verkstead"),
        "verkstead",
        "main",
        Some("work"),
    )
    .await
    .unwrap()
    .unwrap();
    let beside = register_repo_as(
        &pool,
        Path::new("/watched/askance"),
        "askance",
        "main",
        Some("work"),
    )
    .await
    .unwrap()
    .unwrap();

    let drafting = start_conversation(&pool, own.id, "rate-limiting", THIS_DEVICE)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        add_companion(&pool, drafting, beside.id).await.unwrap(),
        Adding::Added,
    );

    assert!(matches!(
        repoint_repo(&pool, own.id, "home").await.unwrap(),
        Repointing::Repointed(_),
    ));
    assert!(matches!(
        repoint_repo(&pool, beside.id, "home").await.unwrap(),
        Repointing::Repointed(_),
    ));

    sqlx::query("INSERT INTO worktrees (conversation_id, path) VALUES (?, ?)")
        .bind(drafting)
        .bind("/state/worktrees/verkstead-rate-limiting")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        repoint_repo(&pool, own.id, "work").await.unwrap(),
        Repointing::InUse {
            on: vec!["rate-limiting".to_owned()],
            beside: vec![],
        },
    );
    assert_eq!(
        repoint_repo(&pool, beside.id, "work").await.unwrap(),
        Repointing::InUse {
            on: vec![],
            beside: vec!["rate-limiting".to_owned()],
        },
    );
}

/// And a Repo nobody has registered, or one taken away, is nothing to repoint.
#[tokio::test]
async fn an_unregistered_repo_is_not_repointed() {
    let (_dir, pool) = fresh_pool().await;

    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();
    unregister_repo(&pool, repo.id).await.unwrap();

    assert_eq!(
        repoint_repo(&pool, repo.id, "home").await.unwrap(),
        Repointing::NoSuchRepo,
    );
    assert_eq!(
        repoint_repo(&pool, 404, "home").await.unwrap(),
        Repointing::NoSuchRepo,
    );
}
