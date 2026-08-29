//! What a Repo remembers about the Pairings it was last grilled with, so the
//! next Conversation started on it arrives with every picker filled.
//!
//! Nothing here looks at the filesystem. Whether a remembered Profile's pair is
//! still where it was left is decided above the store, where the boundary lives
//! — these tests are about what is written down and read back.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_store::{
    AgentType, Deleting, Profile, ProfileFacts, Repo, create_profile, delete_profile,
    open_database, register_repo, remembered_pairings, set_grilling_pairing,
    set_implementation_pairing, set_review_pairing, start_conversation, start_grilling,
    update_profile,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// The model every made-up Profile here lists.
const MODEL: &str = "claude-opus-5";

fn facts(name: &str, models: &[&str]) -> ProfileFacts {
    ProfileFacts {
        name: name.to_owned(),
        claude_dir: PathBuf::from(format!("/watched/accounts/{name}/.claude")),
        config_file: PathBuf::from(format!("/watched/accounts/{name}/.claude.json")),
        models: models.iter().map(|model| (*model).to_owned()).collect(),
        agent_type: AgentType::Claude,
    }
}

async fn saved(pool: &SqlitePool, name: &str) -> Profile {
    create_profile(pool, &facts(name, &[MODEL]))
        .await
        .unwrap()
        .expect("nothing is called that yet")
}

async fn repo(pool: &SqlitePool, name: &str) -> Repo {
    register_repo(
        pool,
        &PathBuf::from(format!("/watched/{name}")),
        name,
        "main",
    )
    .await
    .unwrap()
    .expect("nothing is registered there yet")
}

/// A Conversation on `repo`, grilled under every Pairing — which is the one
/// thing that writes the memory.
async fn grilled(
    pool: &SqlitePool,
    repo: &Repo,
    grilling: &Profile,
    implementation: &Profile,
    review: &Profile,
) {
    let id = start_conversation(pool, repo.id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();

    set_grilling_pairing(pool, id, grilling.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(pool, id, implementation.id, Some(MODEL))
        .await
        .unwrap();
    set_review_pairing(pool, id, review.id, Some(MODEL))
        .await
        .unwrap();

    start_grilling(pool, id, "deadbeef", Path::new("/state/worktrees/x"), &[])
        .await
        .unwrap();
}

/// The whole of it: grilling one Conversation is what fills the next one's
/// pickers.
#[tokio::test]
async fn a_repo_remembers_what_it_was_last_grilled_with() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool, "verkstead").await;
    let fable = saved(&pool, "fable").await;
    let opus = saved(&pool, "opus").await;
    let haiku = saved(&pool, "haiku").await;

    grilled(&pool, &repo, &fable, &opus, &haiku).await;

    let remembered = remembered_pairings(&pool, repo.id).await.unwrap();

    let grilling = remembered.grilling.expect("the grilling half was recorded");
    assert_eq!(grilling.profile.id, fable.id);
    assert_eq!(grilling.model.as_deref(), Some(MODEL));

    let implementation = remembered
        .implementation
        .expect("the implementation half was recorded");
    assert_eq!(implementation.profile.id, opus.id);
    assert_eq!(implementation.model.as_deref(), Some(MODEL));

    let review = remembered.review.expect("and so was the review one");
    assert_eq!(review.profile.id, haiku.id);
    assert_eq!(review.model.as_deref(), Some(MODEL));
}

/// A Repo nobody has grilled anything on has nothing to say, which is what
/// leaves its pickers exactly as they have always been.
#[tokio::test]
async fn a_repo_with_no_memory_remembers_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool, "verkstead").await;

    assert_eq!(
        remembered_pairings(&pool, repo.id).await.unwrap(),
        Default::default()
    );
}

/// Drafting is not grilling. The memory is of what a Conversation actually ran
/// under, so a choice made and left unpressed is not one.
#[tokio::test]
async fn choosing_without_grilling_remembers_nothing() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool, "verkstead").await;
    let fable = saved(&pool, "fable").await;

    let id = start_conversation(&pool, repo.id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();
    set_grilling_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();
    set_review_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();

    assert_eq!(
        remembered_pairings(&pool, repo.id).await.unwrap(),
        Default::default()
    );
}

/// The last one stands. What a Repo remembers is what it was *last* grilled
/// with, not a list of everything it ever was.
#[tokio::test]
async fn grilling_again_replaces_what_was_remembered() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool, "verkstead").await;
    let fable = saved(&pool, "fable").await;
    let opus = saved(&pool, "opus").await;

    grilled(&pool, &repo, &fable, &opus, &fable).await;
    grilled(&pool, &repo, &opus, &fable, &opus).await;

    let remembered = remembered_pairings(&pool, repo.id).await.unwrap();
    assert_eq!(remembered.grilling.unwrap().profile.id, opus.id);
    assert_eq!(remembered.implementation.unwrap().profile.id, fable.id);
    assert_eq!(remembered.review.unwrap().profile.id, opus.id);
}

/// One memory per Repo. Two Repos grilled under different accounts each get
/// their own back.
#[tokio::test]
async fn each_repo_remembers_its_own() {
    let (_dir, pool) = fresh_pool().await;
    let verkstead = repo(&pool, "verkstead").await;
    let askance = repo(&pool, "askance").await;
    let fable = saved(&pool, "fable").await;
    let opus = saved(&pool, "opus").await;

    grilled(&pool, &verkstead, &fable, &fable, &fable).await;
    grilled(&pool, &askance, &opus, &opus, &opus).await;

    assert_eq!(
        remembered_pairings(&pool, verkstead.id)
            .await
            .unwrap()
            .grilling
            .unwrap()
            .profile
            .id,
        fable.id
    );
    assert_eq!(
        remembered_pairings(&pool, askance.id)
            .await
            .unwrap()
            .grilling
            .unwrap()
            .profile
            .id,
        opus.id
    );
}

/// A Profile that has stopped listing the model still comes back, model and
/// all: which of a Profile's models are still real is a question about the
/// Profile, and it is asked above the store where the pair is judged too.
#[tokio::test]
async fn a_model_a_profile_no_longer_lists_still_comes_back_as_it_was_written() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool, "verkstead").await;
    let fable = saved(&pool, "fable").await;

    grilled(&pool, &repo, &fable, &fable, &fable).await;

    update_profile(&pool, fable.id, &facts("fable", &["claude-sonnet-5"]))
        .await
        .unwrap();

    let grilling = remembered_pairings(&pool, repo.id).await.unwrap().grilling;
    let grilling = grilling.expect("the row still names a Profile that is there");
    assert_eq!(grilling.model.as_deref(), Some(MODEL));
    assert_eq!(grilling.profile.models, ["claude-sonnet-5"]);
}
/// The memory never dangles: the Conversation that was grilled under a Profile
/// still names it, so removing that Profile is refused long before the row
/// remembering it could be left pointing at nothing.
#[tokio::test]
async fn a_remembered_profile_cannot_be_removed_out_from_under_the_memory() {
    let (_dir, pool) = fresh_pool().await;
    let repo = repo(&pool, "verkstead").await;
    let fable = saved(&pool, "fable").await;
    let opus = saved(&pool, "opus").await;

    let haiku = saved(&pool, "haiku").await;

    grilled(&pool, &repo, &fable, &opus, &haiku).await;

    assert_eq!(
        delete_profile(&pool, fable.id).await.unwrap(),
        Deleting::InUse
    );
    assert_eq!(
        delete_profile(&pool, opus.id).await.unwrap(),
        Deleting::InUse
    );
    assert_eq!(
        delete_profile(&pool, haiku.id).await.unwrap(),
        Deleting::InUse,
        "the review half is in use exactly as the other two are",
    );

    let remembered = remembered_pairings(&pool, repo.id).await.unwrap();
    assert_eq!(remembered.grilling.unwrap().profile.id, fable.id);
    assert_eq!(remembered.implementation.unwrap().profile.id, opus.id);
    assert_eq!(remembered.review.unwrap().profile.id, haiku.id);
}
