//! Agent Profiles: what saving one records, what removing one refuses, and the
//! two Pairings — a Profile and one of its models — a Conversation chooses
//! before it can be grilled.
//!
//! Nothing here checks a path against the filesystem or against the Watched
//! Paths. That is decided above the store, where the boundary lives — these
//! tests are about what is written down and read back.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_store::{
    AgentType, Chosen, Deleting, Picked, Profile, ProfileFacts, Saving, create_profile,
    delete_profile, load_conversation, load_profile, open_database, profiles, register_repo,
    set_grilling_pairing, set_implementation_pairing, set_review_pairing, skip_review,
    start_conversation, start_grilling, update_profile,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Profile to save, named — the pair is made up, because whether it is really
/// there is not this crate's question.
fn facts(name: &str) -> ProfileFacts {
    ProfileFacts {
        name: name.to_owned(),
        claude_dir: PathBuf::from(format!("/watched/accounts/{name}/.claude")),
        config_file: PathBuf::from(format!("/watched/accounts/{name}/.claude.json")),
        models: vec!["claude-opus-5".to_owned()],
        agent_type: AgentType::Claude,
    }
}

async fn saved(pool: &SqlitePool, name: &str) -> Profile {
    create_profile(pool, &facts(name))
        .await
        .unwrap()
        .expect("nothing is called that yet")
}

/// The model every made-up Profile here lists, which is the one a Pairing is
/// made of unless a test says otherwise.
const MODEL: &str = "claude-opus-5";

/// A Conversation to hang the two choices off.
async fn conversation(pool: &SqlitePool) -> i64 {
    let repo = register_repo(pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    start_conversation(pool, repo.id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn a_saved_profile_holds_its_pair_its_models_and_its_agent_type() {
    let (_dir, pool) = fresh_pool().await;

    let profile = saved(&pool, "work").await;

    assert_eq!(profile.name, "work");
    assert_eq!(
        profile.claude_dir,
        PathBuf::from("/watched/accounts/work/.claude")
    );
    assert_eq!(
        profile.config_file,
        PathBuf::from("/watched/accounts/work/.claude.json")
    );
    assert_eq!(profile.models, ["claude-opus-5"]);

    // One value, and the point of it is that the column is there — the second
    // backend slots in beside `claude` rather than being migrated in under it.
    assert_eq!(profile.agent_type, AgentType::Claude);

    assert_eq!(
        load_profile(&pool, profile.id).await.unwrap(),
        Some(profile)
    );
}

/// The list is the Profile's own: a Profile names what its account can
/// actually launch, and each entry is as good as the next.
#[tokio::test]
async fn a_profile_holds_every_model_it_lists_in_the_order_it_was_given() {
    let (_dir, pool) = fresh_pool().await;

    let mut several = facts("work");
    several.models = vec![
        "claude-opus-5".to_owned(),
        "claude-fable-5".to_owned(),
        "claude-haiku-4-5-20251001".to_owned(),
    ];

    let profile = create_profile(&pool, &several).await.unwrap().unwrap();
    assert_eq!(profile.models, several.models);

    let read = load_profile(&pool, profile.id).await.unwrap().unwrap();
    assert_eq!(read.models, several.models);

    let listed = profiles(&pool).await.unwrap();
    assert_eq!(listed[0].models, several.models);
}

/// The list is replaced rather than added to: it is a handful of lines the human
/// retyped.
#[tokio::test]
async fn rewriting_a_profile_replaces_its_list_rather_than_adding_to_it() {
    let (_dir, pool) = fresh_pool().await;
    let profile = saved(&pool, "work").await;

    let mut shorter = facts("work");
    shorter.models = vec!["claude-fable-5".to_owned(), "claude-opus-5".to_owned()];
    update_profile(&pool, profile.id, &shorter).await.unwrap();

    let mut shorter_still = facts("work");
    shorter_still.models = vec!["claude-opus-5".to_owned()];
    update_profile(&pool, profile.id, &shorter_still)
        .await
        .unwrap();

    let read = load_profile(&pool, profile.id).await.unwrap().unwrap();
    assert_eq!(read.models, ["claude-opus-5"]);
}

/// A Profile saved before the list existed has one model in the old column and
/// no rows in the new table. It reads as the list it always was, with nothing for
/// the human to re-enter.
#[tokio::test]
async fn a_profile_written_before_the_list_reads_its_old_model_as_its_only_one() {
    let (_dir, pool) = fresh_pool().await;

    sqlx::query(
        "INSERT INTO profiles (name, claude_dir, config_file, model, agent_type)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind("work")
    .bind("/watched/accounts/work/.claude")
    .bind("/watched/accounts/work/.claude.json")
    .bind("claude-opus-5")
    .bind("claude")
    .execute(&pool)
    .await
    .unwrap();

    let listed = profiles(&pool).await.unwrap();
    assert_eq!(listed[0].models, ["claude-opus-5"]);
    assert_eq!(
        load_profile(&pool, listed[0].id)
            .await
            .unwrap()
            .unwrap()
            .models,
        ["claude-opus-5"]
    );
}

#[tokio::test]
async fn profiles_are_listed_by_name() {
    let (_dir, pool) = fresh_pool().await;
    saved(&pool, "work").await;
    saved(&pool, "anthropic").await;
    saved(&pool, "personal").await;

    let listed: Vec<String> = profiles(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|profile| profile.name)
        .collect();

    assert_eq!(listed, ["anthropic", "personal", "work"]);
}

/// A picker with two `work` rows in it is a picker nobody can use.
#[tokio::test]
async fn a_name_another_profile_already_has_is_refused() {
    let (_dir, pool) = fresh_pool().await;
    saved(&pool, "work").await;

    assert!(
        create_profile(&pool, &facts("work"))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(profiles(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn everything_about_a_profile_is_the_humans_to_rewrite() {
    let (_dir, pool) = fresh_pool().await;
    let profile = saved(&pool, "work").await;

    let mut changed = facts("anthropic");
    changed.models = vec!["claude-fable-5".to_owned()];

    assert_eq!(
        update_profile(&pool, profile.id, &changed).await.unwrap(),
        Saving::Saved
    );

    let read = load_profile(&pool, profile.id).await.unwrap().unwrap();
    assert_eq!(read.name, "anthropic");
    assert_eq!(read.models, ["claude-fable-5"]);
    assert_eq!(
        read.claude_dir,
        PathBuf::from("/watched/accounts/anthropic/.claude")
    );
}

/// Rewriting a Profile under its own name is not a clash with itself.
#[tokio::test]
async fn a_profile_keeping_its_own_name_is_not_refused() {
    let (_dir, pool) = fresh_pool().await;
    let profile = saved(&pool, "work").await;

    let mut same_name = facts("work");
    same_name.models = vec!["claude-fable-5".to_owned()];

    assert_eq!(
        update_profile(&pool, profile.id, &same_name).await.unwrap(),
        Saving::Saved
    );
    assert_eq!(
        load_profile(&pool, profile.id)
            .await
            .unwrap()
            .unwrap()
            .models,
        ["claude-fable-5"]
    );
}

#[tokio::test]
async fn renaming_a_profile_to_another_ones_name_is_refused() {
    let (_dir, pool) = fresh_pool().await;
    saved(&pool, "work").await;
    let personal = saved(&pool, "personal").await;

    assert_eq!(
        update_profile(&pool, personal.id, &facts("work"))
            .await
            .unwrap(),
        Saving::NameTaken
    );
    assert_eq!(
        load_profile(&pool, personal.id)
            .await
            .unwrap()
            .unwrap()
            .name,
        "personal"
    );
}

#[tokio::test]
async fn rewriting_a_profile_that_is_not_there_says_so() {
    let (_dir, pool) = fresh_pool().await;

    assert_eq!(
        update_profile(&pool, 404, &facts("work")).await.unwrap(),
        Saving::NoSuchProfile
    );
    assert!(load_profile(&pool, 404).await.unwrap().is_none());
}

#[tokio::test]
async fn a_profile_nobody_is_running_under_is_removed() {
    let (_dir, pool) = fresh_pool().await;
    let profile = saved(&pool, "work").await;

    assert_eq!(
        delete_profile(&pool, profile.id).await.unwrap(),
        Deleting::Deleted
    );
    assert!(profiles(&pool).await.unwrap().is_empty());

    assert_eq!(
        delete_profile(&pool, profile.id).await.unwrap(),
        Deleting::NoSuchProfile
    );
}

/// Refused rather than taken away: a Conversation pointing at a Profile that is
/// not there is a session that fails to start with nobody watching.
#[tokio::test]
async fn a_profile_a_conversation_has_chosen_cannot_be_removed() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let grilling = saved(&pool, "fable").await;
    let implementing = saved(&pool, "opus").await;

    set_grilling_pairing(&pool, id, grilling.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, implementing.id, Some(MODEL))
        .await
        .unwrap();

    assert_eq!(
        delete_profile(&pool, grilling.id).await.unwrap(),
        Deleting::InUse
    );
    assert_eq!(
        delete_profile(&pool, implementing.id).await.unwrap(),
        Deleting::InUse
    );
    assert_eq!(profiles(&pool).await.unwrap().len(), 2);
}

/// The two are separate choices because they are genuinely separate accounts and
/// models — grill on fable, implement on opus.
#[tokio::test]
async fn a_conversation_chooses_its_two_pairings_independently() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let fable = saved(&pool, "fable").await;
    let opus = saved(&pool, "opus").await;

    assert_eq!(
        set_grilling_pairing(&pool, id, fable.id, Some(MODEL))
            .await
            .unwrap(),
        Chosen::Chosen
    );

    // One chosen and not the other, which is the state the next stage refuses to
    // grill from.
    let half = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        half.grilling_pairing.as_ref().map(|p| p.profile.id),
        Some(fable.id)
    );
    assert_eq!(half.implementation_pairing, None);

    assert_eq!(
        set_implementation_pairing(&pool, id, opus.id, Some(MODEL))
            .await
            .unwrap(),
        Chosen::Chosen
    );

    let both = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        both.grilling_pairing.map(|p| p.profile.name),
        Some("fable".to_owned())
    );
    assert_eq!(
        both.implementation_pairing.map(|p| p.profile.name),
        Some("opus".to_owned())
    );
}

/// Both halves of the choice come back, and the model is the one that was
/// paired rather than whatever the Profile lists first.
#[tokio::test]
async fn a_pairing_holds_the_model_it_was_chosen_with() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let profile = create_profile(
        &pool,
        &ProfileFacts {
            models: vec!["claude-opus-5".to_owned(), "claude-fable-5".to_owned()],
            ..facts("work")
        },
    )
    .await
    .unwrap()
    .unwrap();

    set_grilling_pairing(&pool, id, profile.id, Some("claude-fable-5"))
        .await
        .unwrap();

    let pairing = load_conversation(&pool, id)
        .await
        .unwrap()
        .unwrap()
        .grilling_pairing
        .unwrap();

    assert_eq!(pairing.model.as_deref(), Some("claude-fable-5"));
    assert_eq!(pairing.runs_on(), Some("claude-fable-5"));

    // And choosing again replaces both halves rather than leaving the old model
    // beside the new Profile.
    set_grilling_pairing(&pool, id, profile.id, Some("claude-opus-5"))
        .await
        .unwrap();

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .grilling_pairing
            .and_then(|pairing| pairing.model),
        Some("claude-opus-5".to_owned())
    );
}

/// A Conversation that chose a Profile before there was a model to choose
/// beside it goes on running what that Profile carries — which is what carries
/// every Conversation started before pairings existed across.
#[tokio::test]
async fn a_profile_chosen_before_pairings_runs_on_the_model_it_carried() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let profile = saved(&pool, "work").await;

    // The column alone, which is the whole of what an old choice wrote.
    sqlx::query("UPDATE conversations SET grilling_profile_id = ? WHERE id = ?")
        .bind(profile.id)
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();

    let pairing = load_conversation(&pool, id)
        .await
        .unwrap()
        .unwrap()
        .grilling_pairing
        .unwrap();

    assert_eq!(pairing.model, None);
    assert_eq!(pairing.runs_on(), Some(MODEL));
}

/// Both Pairings are fixed when grilling starts, the way the branch, the base
/// commit and the Brief are: what runs the work is settled before the work
/// begins rather than swapped underneath it.
#[tokio::test]
async fn a_pairing_cannot_be_chosen_once_grilling_has_started() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let fable = saved(&pool, "fable").await;
    let opus = saved(&pool, "opus").await;

    set_grilling_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();

    start_grilling(
        &pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        Path::new("/var/lib/verkstead/worktrees/verkstead-amber-kestrel"),
        &[],
    )
    .await
    .unwrap();

    assert_eq!(
        set_grilling_pairing(&pool, id, opus.id, Some(MODEL))
            .await
            .unwrap(),
        Chosen::NotDrafting
    );
    assert_eq!(
        set_implementation_pairing(&pool, id, opus.id, Some(MODEL))
            .await
            .unwrap(),
        Chosen::NotDrafting
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        conversation.grilling_pairing.map(|p| p.profile.name),
        Some("fable".to_owned())
    );
    assert_eq!(
        conversation.implementation_pairing.map(|p| p.profile.name),
        Some("fable".to_owned())
    );
}

/// The same Profile may fill both roles: they are roles a Profile is used in,
/// not kinds of Profile.
#[tokio::test]
async fn one_profile_can_be_both_of_a_conversations_choices() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let only = saved(&pool, "work").await;

    set_grilling_pairing(&pool, id, only.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, only.id, Some(MODEL))
        .await
        .unwrap();

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        conversation.grilling_pairing.map(|p| p.profile.id),
        Some(only.id)
    );
    assert_eq!(
        conversation.implementation_pairing.map(|p| p.profile.id),
        Some(only.id)
    );
}

/// A column naming a Profile that is not there would be a session that fails to
/// start later, so the choice is refused now.
#[tokio::test]
async fn a_profile_that_is_not_there_cannot_be_chosen() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    assert_eq!(
        set_grilling_pairing(&pool, id, 404, Some(MODEL))
            .await
            .unwrap(),
        Chosen::NoSuchProfile
    );
    assert_eq!(
        set_implementation_pairing(&pool, id, 404, Some(MODEL))
            .await
            .unwrap(),
        Chosen::NoSuchProfile
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.grilling_pairing, None);
    assert_eq!(conversation.implementation_pairing, None);
}

#[tokio::test]
async fn choosing_on_a_conversation_that_is_not_there_says_so() {
    let (_dir, pool) = fresh_pool().await;
    let profile = saved(&pool, "work").await;

    assert_eq!(
        set_grilling_pairing(&pool, 404, profile.id, Some(MODEL))
            .await
            .unwrap(),
        Chosen::NoSuchConversation
    );
    assert_eq!(
        set_implementation_pairing(&pool, 404, profile.id, Some(MODEL))
            .await
            .unwrap(),
        Chosen::NoSuchConversation
    );
}

/// A Conversation starts with neither, which is what makes it not ready to
/// grill: the human chooses both before the next stage will run anything.
#[tokio::test]
async fn a_started_conversation_has_chosen_neither_pairing() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(conversation.grilling_pairing, None);
    assert_eq!(conversation.implementation_pairing, None);
}

/// The point of it being in SQLite rather than in a page's memory: the server is
/// a service that restarts, and what the human settled must be there afterwards.
#[tokio::test]
async fn profiles_and_the_choices_made_of_them_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("verkstead.db");

    let pool = open_database(&database).await.unwrap();
    let id = conversation(&pool).await;
    let fable = saved(&pool, "fable").await;
    let opus = saved(&pool, "opus").await;
    set_grilling_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, opus.id, Some(MODEL))
        .await
        .unwrap();
    pool.close().await;

    let pool = open_database(&database).await.unwrap();

    assert_eq!(profiles(&pool).await.unwrap().len(), 2);
    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    assert_eq!(
        conversation.grilling_pairing.map(|p| p.profile.name),
        Some("fable".to_owned())
    );
    assert_eq!(
        conversation
            .implementation_pairing
            .map(|p| (p.profile.models, p.model)),
        Some((
            vec!["claude-opus-5".to_owned()],
            Some("claude-opus-5".to_owned())
        ))
    );
}

#[tokio::test]
async fn nothing_saved_means_nothing_listed() {
    let (_dir, pool) = fresh_pool().await;

    assert!(profiles(&pool).await.unwrap().is_empty());
}

/// *No review* is a choice, stored apart from having chosen nothing.
///
/// Which is the whole of why it is not simply the absence of a Pairing: a
/// picker nobody has touched and a picker moved to the row that runs nothing
/// leave the same column empty, and only one of them lets the work start.
#[tokio::test]
async fn no_review_is_a_choice_rather_than_an_empty_picker() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .review_pairing,
        Picked::Nothing,
        "nothing has been picked yet",
    );

    assert_eq!(skip_review(&pool, id).await.unwrap(), Chosen::Chosen);

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .review_pairing,
        Picked::Skipped,
        "and now something has",
    );
}

/// The two rows are rows of one list, so picking either unpicks the other.
#[tokio::test]
async fn picking_a_review_pairing_and_picking_none_replace_each_other() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let fable = saved(&pool, "fable").await;

    set_review_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();
    skip_review(&pool, id).await.unwrap();

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .review_pairing,
        Picked::Skipped,
        "the account it was going to be reviewed under is off it",
    );

    set_review_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();

    let picked = load_conversation(&pool, id)
        .await
        .unwrap()
        .unwrap()
        .review_pairing;

    assert_eq!(
        picked.pairing().map(|pairing| pairing.profile.id),
        Some(fable.id),
        "and picking an account back takes the row that ran nothing away",
    );
}

/// And it is fixed when grilling starts, exactly as the Pairings beside it are.
#[tokio::test]
async fn no_review_cannot_be_picked_once_grilling_has_started() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let fable = saved(&pool, "fable").await;

    set_grilling_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();
    set_review_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();

    start_grilling(
        &pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        Path::new("/var/lib/verkstead/worktrees/verkstead-amber-kestrel"),
        &[],
    )
    .await
    .unwrap();

    assert_eq!(skip_review(&pool, id).await.unwrap(), Chosen::NotDrafting);

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .review_pairing
            .pairing()
            .map(|pairing| pairing.profile.id),
        Some(fable.id),
        "and what it was grilled under is exactly where it was",
    );
}
