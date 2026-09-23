//! The pending steer: the form the human is part-way through, kept beside the
//! Conversation until they decide.
//!
//! Pressing Steer writes one and the workbench draws it as the last item on the
//! Timeline; the form that was a modal is its details pane. So the row is what
//! makes a steer something to come back to — from another item, another
//! Conversation, another device, or after a reload — rather than something that
//! has to be written in one sitting.
//!
//! One per Conversation, and the second press finds the first: a Conversation
//! already holding a half-written form is not one to start another beside.
//!
//! And it goes two ways, both of them the human deciding. Cancel discards it and
//! leaves the Conversation stopped; a submit discards it in the transaction that
//! writes the Steer Event it became, so the record and the form it was filled in
//! on are never both on the page.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_store::{
    Account, CompanionMode, Lifecycle, Pending, PendingAddition, PendingForm, PendingPairing,
    PendingUpgrade, ProfileFacts, Steer, Steering, create_profile, discard_pending_steer,
    open_database, open_pending_steer, pending_steer, register_repo, save_brief,
    save_pending_steer, start_conversation, steer_conversation, timeline,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Conversation to press Steer on. Still drafting, which is a source for a
/// steer like any other.
async fn drafting(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/srv/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .expect("nothing is registered at that path yet");

    let id = start_conversation(pool, repo.id, "rate-limiting")
        .await
        .unwrap()
        .expect("the Repo was just registered");

    save_brief(pool, id, "# Rate limiting\n").await.unwrap();

    id
}

/// An Agent Profile to pair with, by name and with the models it lists.
async fn profile(pool: &SqlitePool, name: &str, models: &[&str]) -> i64 {
    create_profile(
        pool,
        &ProfileFacts {
            name: Some(name.to_owned()),
            account: Account::Claude {
                claude_dir: PathBuf::from(format!("/state/profiles/{name}")),
                config_file: PathBuf::from(format!("/state/profiles/{name}.json")),
            },
            models: models.iter().map(|model| (*model).to_owned()).collect(),
            memory: true,
        },
    )
    .await
    .unwrap()
    .expect("nothing is called that yet")
    .id
}

/// The plainest steer there is: the move and nothing beside it.
fn into(target: Lifecycle) -> Steer<'static> {
    Steer {
        target,
        pairings: &[],
        brief: None,
        instruction: None,
        direction: None,
        worktree: None,
        base: None,
        companions: &[],
        opened: &[],
        checkouts: &[],
        said: None,
    }
}

/// The press writes the row, and every slot on it opens empty: nothing picked,
/// nothing written, nothing ticked. What fills them is the form saving itself as
/// it is typed, and empty is what it opens on.
#[tokio::test]
async fn a_press_opens_one_with_every_slot_empty() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    assert_eq!(
        open_pending_steer(&pool, id).await.unwrap(),
        Pending::Opened
    );

    let pending = pending_steer(&pool, id)
        .await
        .unwrap()
        .expect("the press just wrote one");

    assert!(!pending.at.is_empty(), "it says when it was opened");
    assert_eq!(
        pending.form,
        PendingForm::default(),
        "no target picked, nothing written, nothing ticked and no companion row touched",
    );
}

/// A Conversation nobody is steering has none, which is what says the workbench
/// draws no item at the end of its Timeline.
#[tokio::test]
async fn an_unsteered_conversation_has_none() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    assert_eq!(pending_steer(&pool, id).await.unwrap(), None);
}

/// The second press makes nothing and reports the one there is. A Conversation
/// already carrying a half-written form is not one to start another beside — the
/// press selects what is there.
#[tokio::test]
async fn a_second_press_finds_the_one_there_is() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    let first = open_pending_steer(&pool, id).await.unwrap();
    let opened = pending_steer(&pool, id).await.unwrap().unwrap().at;

    assert_eq!(first, Pending::Opened);
    assert_eq!(
        open_pending_steer(&pool, id).await.unwrap(),
        Pending::Standing
    );
    assert_eq!(
        pending_steer(&pool, id).await.unwrap().unwrap().at,
        opened,
        "the row is left exactly as it stands, down to when it was opened",
    );
}

/// And a press on a Conversation that is not there says so rather than writing a
/// row attributed to nobody.
#[tokio::test]
async fn a_press_on_nothing_says_so() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        open_pending_steer(&pool, 404).await.unwrap(),
        Pending::NoSuchConversation
    );
    assert_eq!(pending_steer(&pool, 404).await.unwrap(), None);
}

/// Cancel: the row goes, and the press says there was one. Cancelling decides
/// nothing about the Conversation — it is left stopped, with Resume on offer —
/// so there is nothing else here to undo.
#[tokio::test]
async fn cancelling_discards_it() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    open_pending_steer(&pool, id).await.unwrap();

    assert!(discard_pending_steer(&pool, id).await.unwrap());
    assert_eq!(pending_steer(&pool, id).await.unwrap(), None);
    assert!(
        !discard_pending_steer(&pool, id).await.unwrap(),
        "a cancel landing behind another device's is an outcome rather than a failure",
    );
}

/// And the submit takes it away in the transaction that writes the record it
/// became: the Steer Event and the Moved line land, and the form they were
/// filled in on is gone.
#[tokio::test]
async fn submitting_discards_it_with_the_record() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    open_pending_steer(&pool, id).await.unwrap();

    assert_eq!(
        steer_conversation(&pool, id, into(Lifecycle::Implementing))
            .await
            .unwrap(),
        Steering::Steered
    );

    assert_eq!(pending_steer(&pool, id).await.unwrap(), None);

    let kinds = timeline(&pool, id).await.unwrap();
    assert!(
        kinds.iter().any(|event| matches!(
            event.event,
            verkstead_store::Event::Steer(Lifecycle::Implementing, None)
        )),
        "the record it became is on the Timeline",
    );
}

/// A steer submitted on a Conversation nobody had a form open on is unremarkable
/// — most steers before this were exactly that, and a database holding no row
/// for one is not a submit to refuse.
#[tokio::test]
async fn submitting_without_one_is_an_ordinary_steer() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    assert_eq!(
        steer_conversation(&pool, id, into(Lifecycle::Done))
            .await
            .unwrap(),
        Steering::Steered
    );
    assert_eq!(pending_steer(&pool, id).await.unwrap(), None);
}

/// Every slot the form has round-trips through a save, and a save carries the
/// whole form: what comes back is what went in, down to the companion rows.
#[tokio::test]
async fn a_save_writes_the_whole_form_and_reads_it_back() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;
    let alongside = register_repo(&pool, Path::new("/srv/askance"), "askance", "main")
        .await
        .unwrap()
        .unwrap();
    let beside = register_repo(&pool, Path::new("/srv/granit"), "granit", "trunk")
        .await
        .unwrap()
        .unwrap();
    let account = profile(&pool, "Work", &["opus"]).await;

    open_pending_steer(&pool, id).await.unwrap();

    let filled = PendingForm {
        target: Some(Lifecycle::Implementing),
        brief: Some("# A second round\n".to_owned()),
        digest: true,
        instruction: Some("Take the modal out".to_owned()),
        follow_up: Some("And say what came of it".to_owned()),
        pairing: Some(PendingPairing {
            profile_id: account,
            model: "opus".to_owned(),
        }),
        interrupt: true,
        added: vec![PendingAddition {
            repo_id: alongside.id,
            mode: CompanionMode::ReadWrite,
            base_ref: Some("release".to_owned()),
            branch: "steer-pane".to_owned(),
        }],
        upgraded: vec![PendingUpgrade {
            repo_id: beside.id,
            branch: String::new(),
        }],
    };

    assert!(save_pending_steer(&pool, id, &filled).await.unwrap());

    let pending = pending_steer(&pool, id).await.unwrap().unwrap();
    assert_eq!(pending.form, filled);

    // And the next save is the whole form again, so a row unticked leaves rather
    // than lingering beside what the human can see.
    assert!(
        save_pending_steer(&pool, id, &PendingForm::default())
            .await
            .unwrap()
    );
    assert_eq!(
        pending_steer(&pool, id).await.unwrap().unwrap().form,
        PendingForm::default()
    );
}

/// And a save against a pending steer that has gone says so rather than writing
/// a row nobody opened: a submit or a cancel from another device is what has
/// usually landed in front of it.
#[tokio::test]
async fn a_save_without_one_says_so() {
    let (_dir, pool) = fresh_pool().await;
    let id = drafting(&pool).await;

    assert!(
        !save_pending_steer(&pool, id, &PendingForm::default())
            .await
            .unwrap()
    );
    assert_eq!(pending_steer(&pool, id).await.unwrap(), None);
}
