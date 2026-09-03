//! Agent Profiles: what saving one records, what removing one takes with it,
//! and the two Pairings — a Profile and one of its models — a Conversation
//! chooses before it can be grilled.
//!
//! Nothing here checks a path against the filesystem or against the Watched
//! Paths. That is decided above the store, where the boundary lives — these
//! tests are about what is written down and read back.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;
use verkstead_schema::Direction;
use verkstead_store::{
    Account, AgentType, Chosen, Deleting, Event, Lifecycle, Pairing, Picked, Profile, ProfileFacts,
    Saving, create_profile, delete_profile, load_conversation, load_profile, open_database,
    profiles, register_repo, set_grilling_pairing, set_implementation_pairing, set_review_pairing,
    skip_grilling, skip_review, start_building, start_capture, start_conversation, start_grilling,
    timeline, update_profile,
};

/// A pool over a fresh database, plus the directory keeping it alive.
async fn fresh_pool() -> (tempfile::TempDir, SqlitePool) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_database(&dir.path().join("verkstead.db"))
        .await
        .unwrap();
    (dir, pool)
}

/// A Profile to save, named — the account is made up, because whether it is
/// really there is not this crate's question.
fn facts(name: &str) -> ProfileFacts {
    facts_for(name, claude(name))
}

/// And the same with an account of some other type, for the Profiles that are
/// about the shape rather than about the name.
fn facts_for(name: &str, account: Account) -> ProfileFacts {
    ProfileFacts {
        name: name.to_owned(),
        account,
        models: vec!["claude-opus-5".to_owned()],
    }
}

/// The Claude account those Profiles run as: the pair, under the account's own
/// directory.
fn claude(name: &str) -> Account {
    Account::Claude {
        claude_dir: PathBuf::from(format!("/watched/accounts/{name}/.claude")),
        config_file: PathBuf::from(format!("/watched/accounts/{name}/.claude.json")),
    }
}

/// And a Codex one: the single home its whole account is kept under.
fn codex(name: &str) -> Account {
    Account::Codex {
        home: PathBuf::from(format!("/watched/accounts/{name}/.codex")),
    }
}

/// And a Grok Build one, which is the same shape under a directory of its own.
fn grok(name: &str) -> Account {
    Account::Grok {
        home: PathBuf::from(format!("/watched/accounts/{name}/.grok")),
    }
}

/// And an OpenCode one, which keeps one home too — the directory opencode's
/// XDG paths resolve inside rather than a dot-directory of its own.
fn opencode(name: &str) -> Account {
    Account::OpenCode {
        home: PathBuf::from(format!("/watched/accounts/{name}/opencode")),
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
async fn a_saved_profile_holds_its_account_and_its_models() {
    let (_dir, pool) = fresh_pool().await;

    let profile = saved(&pool, "work").await;

    assert_eq!(profile.name, "work");
    assert_eq!(
        profile.account,
        Account::Claude {
            claude_dir: PathBuf::from("/watched/accounts/work/.claude"),
            config_file: PathBuf::from("/watched/accounts/work/.claude.json"),
        }
    );
    assert_eq!(profile.models, ["claude-opus-5"]);

    // One value, and the point of it is that the column is there and the shape
    // hangs off it — the second backend slots in beside `claude` rather than
    // being migrated in under it.
    assert_eq!(profile.agent_type(), AgentType::Claude);

    assert_eq!(
        load_profile(&pool, profile.id).await.unwrap(),
        Some(profile)
    );
}

/// A Profile of a type this binary does not have is a database written by a
/// newer Verkstead. Refused by the word that is in the column, rather than read
/// past as though it were a Claude row with a pair it has not got.
#[tokio::test]
async fn a_profile_naming_an_agent_type_this_binary_has_not_got_is_refused_by_name() {
    let (_dir, pool) = fresh_pool().await;

    sqlx::query(
        "INSERT INTO profiles (name, claude_dir, config_file, model, agent_type)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind("work")
    .bind("")
    .bind("")
    .bind("grok-code")
    .bind("grok")
    .execute(&pool)
    .await
    .unwrap();

    let refused = profiles(&pool).await.unwrap_err().to_string();
    assert!(
        refused.contains("grok"),
        "the refusal should name the word it did not know, and said {refused:?}"
    );
}

/// And a Profile of a type whose whole account is one home, with no home
/// recorded, is refused the same way.
///
/// Nothing above the store can write one: a home is put down with the Profile
/// and taken away with it. So what this catches is a row somebody edited by
/// hand — and reading it past as a home of the empty string would be a bind
/// landing on `/`.
#[tokio::test]
async fn a_profile_whose_account_is_a_home_it_has_not_got_is_refused() {
    let (_dir, pool) = fresh_pool().await;

    sqlx::query(
        "INSERT INTO profiles (name, claude_dir, config_file, model, agent_type)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind("work")
    .bind("")
    .bind("")
    .bind("gpt-5-codex")
    .bind("codex")
    .execute(&pool)
    .await
    .unwrap();

    let refused = profiles(&pool).await.unwrap_err().to_string();
    assert!(
        refused.contains("home"),
        "the refusal should say what is missing, and said {refused:?}"
    );
}

/// The table the account of every type after Claude lives in: one home per
/// Profile, keyed by its id.
#[tokio::test]
async fn a_profiles_single_home_directory_has_a_table_of_its_own() {
    let (_dir, pool) = fresh_pool().await;
    let profile = saved(&pool, "work").await;

    keep_home(&pool, profile.id, "/watched/accounts/work").await;

    let home: (String,) = sqlx::query_as("SELECT home FROM profile_homes WHERE profile_id = ?")
        .bind(profile.id)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(home.0, "/watched/accounts/work");
}

/// A Profile whose account is one home round-trips whole: saved with it, listed
/// with it, loaded with it, and read back as its own type.
///
/// The account's shape is what says the type, so what this settles is that a
/// row written as `codex` comes back as a Codex account and not as a pair of
/// empty strings — the two path columns are NOT NULL and it has nothing to say
/// in them.
#[tokio::test]
async fn a_profile_whose_account_is_one_home_round_trips_with_it() {
    let (_dir, pool) = fresh_pool().await;

    let saved = create_profile(&pool, &facts_for("work", codex("work")))
        .await
        .unwrap()
        .expect("nothing is called that yet");

    assert_eq!(saved.account, codex("work"));
    assert_eq!(saved.agent_type(), AgentType::Codex);

    let loaded = load_profile(&pool, saved.id)
        .await
        .unwrap()
        .expect("the Profile was just saved");
    assert_eq!(loaded, saved, "one Profile read back is the one written");

    assert_eq!(
        profiles(&pool).await.unwrap(),
        vec![saved],
        "and so is the same Profile read off the list"
    );
}

/// And the types that each keep one home are told apart by the column rather
/// than by the shape they share.
///
/// What is written down is the same in all of them — nothing in the pair
/// columns, one row in `profile_homes` — so the word in `agent_type` is the
/// whole of what says which account a home is. A Grok Profile read back as a
/// Codex one would be a session launched on the wrong binary under the right
/// directory, and an OpenCode one is a third word over the same two columns.
#[tokio::test]
async fn the_types_that_each_keep_one_home_read_back_as_themselves() {
    let (_dir, pool) = fresh_pool().await;

    create_profile(&pool, &facts_for("work", codex("work")))
        .await
        .unwrap()
        .expect("nothing is called that yet");

    let saved = create_profile(&pool, &facts_for("xai", grok("xai")))
        .await
        .unwrap()
        .expect("nothing is called that yet");

    let latest = create_profile(&pool, &facts_for("zen", opencode("zen")))
        .await
        .unwrap()
        .expect("nothing is called that yet");

    assert_eq!(saved.account, grok("xai"));
    assert_eq!(saved.agent_type(), AgentType::Grok);

    assert_eq!(latest.account, opencode("zen"));
    assert_eq!(latest.agent_type(), AgentType::OpenCode);

    for profile in [&saved, &latest] {
        let loaded = load_profile(&pool, profile.id)
            .await
            .unwrap()
            .expect("the Profile was just saved");
        assert_eq!(&loaded, profile, "one Profile read back is the one written");
    }

    assert_eq!(
        profiles(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|profile| profile.agent_type())
            .collect::<Vec<_>>(),
        vec![AgentType::Codex, AgentType::Grok, AgentType::OpenCode],
        "and each of them off the list is its own type"
    );
}

/// Rewriting one replaces the home rather than adding a second, which is what a
/// table keyed by profile id would otherwise refuse on the second save.
#[tokio::test]
async fn rewriting_a_profile_with_a_home_replaces_the_home() {
    let (_dir, pool) = fresh_pool().await;

    let saved = create_profile(&pool, &facts_for("work", codex("work")))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        update_profile(&pool, saved.id, &facts_for("work", codex("moved")))
            .await
            .unwrap(),
        Saving::Saved
    );
    assert_eq!(homes(&pool).await, 1, "one home, the one it is under now");
    assert_eq!(
        load_profile(&pool, saved.id)
            .await
            .unwrap()
            .unwrap()
            .account,
        codex("moved"),
    );
}

/// And removing one takes the home with it, as it takes the models.
#[tokio::test]
async fn removing_a_profile_with_a_home_takes_the_home_it_wrote() {
    let (_dir, pool) = fresh_pool().await;

    let saved = create_profile(&pool, &facts_for("work", codex("work")))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        delete_profile(&pool, saved.id).await.unwrap(),
        Deleting::Deleted
    );
    assert_eq!(homes(&pool).await, 0);
}

/// A Profile whose type keeps no home writes none: an installation with nothing
/// but Claude Profiles has the empty table it always had.
#[tokio::test]
async fn a_claude_profile_writes_no_home_at_all() {
    let (_dir, pool) = fresh_pool().await;

    let saved = saved(&pool, "work").await;
    assert_eq!(homes(&pool).await, 0);

    assert_eq!(
        update_profile(&pool, saved.id, &facts("work"))
            .await
            .unwrap(),
        Saving::Saved
    );
    assert_eq!(homes(&pool).await, 0);
}

/// And removing a Profile takes the home with it.
///
/// `profile_homes` references `profiles(id)` and foreign keys are enforced, so
/// a home left behind is not an orphan row but a Profile that cannot be removed
/// at all — which is the stage that first writes one finding removal broken
/// rather than finding it arranged.
#[tokio::test]
async fn removing_a_profile_takes_the_home_it_kept_its_account_under_with_it() {
    let (_dir, pool) = fresh_pool().await;
    let profile = saved(&pool, "work").await;

    keep_home(&pool, profile.id, "/watched/accounts/work").await;

    assert_eq!(
        delete_profile(&pool, profile.id).await.unwrap(),
        Deleting::Deleted
    );
    assert_eq!(
        homes(&pool).await,
        0,
        "the home goes with the Profile that kept its account under it"
    );
}

/// And rewriting one replaces it, as it replaces the model list: the account is
/// the type's shape, so the home a Profile had is the old type's if the type has
/// changed.
#[tokio::test]
async fn rewriting_a_profile_replaces_the_home_its_account_was_under() {
    let (_dir, pool) = fresh_pool().await;
    let profile = saved(&pool, "work").await;

    keep_home(&pool, profile.id, "/watched/accounts/work").await;

    assert_eq!(
        update_profile(&pool, profile.id, &facts("work"))
            .await
            .unwrap(),
        Saving::Saved
    );
    assert_eq!(
        homes(&pool).await,
        0,
        "a rewrite writes the account whole rather than reconciling it"
    );
}

/// A home written straight into the table, standing in for the backend that
/// will keep its whole account under one: there is no type with a home yet, so
/// nothing above the store can put a row here.
async fn keep_home(pool: &SqlitePool, id: i64, home: &str) {
    sqlx::query("INSERT INTO profile_homes (profile_id, home) VALUES (?, ?)")
        .bind(id)
        .bind(home)
        .execute(pool)
        .await
        .expect("the table is there to be written to");
}

/// And how many the table is holding.
async fn homes(pool: &SqlitePool) -> i64 {
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM profile_homes")
        .fetch_one(pool)
        .await
        .unwrap();

    count
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
/// no rows in the new table. It reads as the list it always was, and as the
/// Claude account it always was, with nothing for the human to re-enter.
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
    assert_eq!(listed[0].account, claude("work"));

    let read = load_profile(&pool, listed[0].id).await.unwrap().unwrap();
    assert_eq!(read.models, ["claude-opus-5"]);
    assert_eq!(read.account, claude("work"));
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
    assert_eq!(read.account, claude("anthropic"));
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

/// Taken away rather than refused, and the Conversation that had chosen it is
/// nulled out: a Profile is always the human's to be finished with, and what it
/// costs them is a picker to fill in again rather than a removal they cannot
/// make.
///
/// One of the two Profiles here, so that the other says what is *not* touched:
/// a role that named some other account is left exactly as it was.
#[tokio::test]
async fn a_profile_a_conversation_has_chosen_is_removed_and_nulled_out_of_it() {
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
        Deleting::Deleted
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();

    assert_eq!(
        conversation.grilling_pairing,
        Picked::Nothing,
        "the role that named it reads as one nothing has been picked for",
    );
    assert_eq!(
        conversation
            .implementation_pairing
            .map(|pairing| pairing.profile.id),
        Some(implementing.id),
        "and the role that named another account is untouched",
    );

    assert_eq!(
        profiles(&pool).await.unwrap().len(),
        1,
        "the Profile itself is gone",
    );
}

/// Every role, not the two a Conversation must have: the review's column is
/// nulled exactly as the other two are.
#[tokio::test]
async fn removing_a_profile_nulls_it_out_of_all_three_roles() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let one = saved(&pool, "work").await;

    set_grilling_pairing(&pool, id, one.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, one.id, Some(MODEL))
        .await
        .unwrap();
    set_review_pairing(&pool, id, one.id, Some(MODEL))
        .await
        .unwrap();

    assert_eq!(
        delete_profile(&pool, one.id).await.unwrap(),
        Deleting::Deleted
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();

    assert_eq!(conversation.grilling_pairing, Picked::Nothing);
    assert_eq!(conversation.implementation_pairing, None);
    assert_eq!(conversation.review_pairing, Picked::Nothing);
}

/// The model half goes with the Profile half. A Pairing is both, so a role left
/// holding a model and no account would be half a choice nothing can launch —
/// and it would prefill the picker with a model the next Profile may not list.
#[tokio::test]
async fn removing_a_profile_takes_the_model_paired_with_it_too() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let going = saved(&pool, "fable").await;
    let staying = saved(&pool, "opus").await;

    set_grilling_pairing(&pool, id, going.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, staying.id, Some(MODEL))
        .await
        .unwrap();

    delete_profile(&pool, going.id).await.unwrap();

    let paired: Vec<(String,)> =
        sqlx::query_as("SELECT role FROM pairing_models WHERE conversation_id = ?")
            .bind(id)
            .fetch_all(&pool)
            .await
            .unwrap();

    assert_eq!(
        paired
            .iter()
            .map(|(role,)| role.as_str())
            .collect::<Vec<_>>(),
        ["implementation"],
        "the going Profile's model row goes with it and the staying one's stays",
    );
}

/// Past drafting is where this matters most: a Conversation whose Pairings were
/// fixed when its work started is exactly the one that cannot re-choose, and
/// removing its Profile is still allowed.
///
/// What it leaves is a Conversation whose next session starts nothing, to be
/// rescued with a steer — which is the cost the removal is worth paying.
#[tokio::test]
async fn a_profile_a_started_conversation_runs_under_is_removed_all_the_same() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let profile = saved(&pool, "work").await;

    set_grilling_pairing(&pool, id, profile.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, profile.id, Some(MODEL))
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
        delete_profile(&pool, profile.id).await.unwrap(),
        Deleting::Deleted
    );

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();

    assert_eq!(conversation.state, Lifecycle::Grilling);
    assert_eq!(conversation.grilling_pairing, Picked::Nothing);
    assert_eq!(conversation.implementation_pairing, None);
}

/// A role the human picked away is not what a removal leaves behind.
///
/// *No review* is something they chose and *no Profile any more* is not, so the
/// row that runs no session is neither written by a removal nor taken away by
/// one: a Conversation that had picked the review away still has, and one that
/// had paired it reads as unpicked rather than as having turned it off.
#[tokio::test]
async fn a_removal_neither_writes_nor_clears_the_row_that_runs_no_session() {
    let (_dir, pool) = fresh_pool().await;
    let repo = register_repo(&pool, Path::new("/watched/verkstead"), "verkstead", "main")
        .await
        .unwrap()
        .unwrap();

    let kept = start_conversation(&pool, repo.id, "amber-kestrel")
        .await
        .unwrap()
        .unwrap();
    let paired = start_conversation(&pool, repo.id, "russet-heron")
        .await
        .unwrap()
        .unwrap();

    let profile = saved(&pool, "work").await;

    skip_review(&pool, kept).await.unwrap();
    set_grilling_pairing(&pool, kept, profile.id, Some(MODEL))
        .await
        .unwrap();

    set_review_pairing(&pool, paired, profile.id, Some(MODEL))
        .await
        .unwrap();

    delete_profile(&pool, profile.id).await.unwrap();

    assert_eq!(
        load_conversation(&pool, kept)
            .await
            .unwrap()
            .unwrap()
            .review_pairing,
        Picked::Skipped,
        "the choice they made stands",
    );
    assert_eq!(
        load_conversation(&pool, paired)
            .await
            .unwrap()
            .unwrap()
            .review_pairing,
        Picked::Nothing,
        "and losing an account is not making that choice for them",
    );
}

/// What has already run is not rewritten by a removal.
///
/// A session's record keeps the Profile's *name* rather than its id, so the
/// Timeline goes on saying what ran under what — which is the whole reason it
/// was written down as a copy.
#[tokio::test]
async fn removing_a_profile_leaves_what_ran_under_it_on_the_record() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let profile = saved(&pool, "work").await;

    set_grilling_pairing(&pool, id, profile.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, profile.id, Some(MODEL))
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

    start_capture(
        &pool,
        id,
        Some("a-session"),
        Some(&Pairing {
            profile: profile.clone(),
            model: Some(MODEL.to_owned()),
        }),
    )
    .await
    .unwrap();

    delete_profile(&pool, profile.id).await.unwrap();

    let ran = timeline(&pool, id)
        .await
        .unwrap()
        .into_iter()
        .find_map(|on| match on.event {
            Event::AgentOutput(_, ran) => Some(ran),
            _ => None,
        })
        .expect("the session it ran is on the Timeline");

    assert_eq!(
        ran.expect("and it says what it ran under").profile,
        "work",
        "a name written down is a name a removal cannot take",
    );
}

/// Nothing anywhere still names a removed Profile, and the schema is what says
/// where to look.
///
/// The walk is SQLite's own answer rather than a list written here: a table
/// added next year that references `profiles` and is not reached by the removal
/// fails this rather than failing a human years later, when the foreign key
/// refuses a removal they cannot see the reason for. Every table it finds is
/// filled first, so a table reached by the walk and never written in a test is
/// not a walk nobody has run.
#[tokio::test]
async fn a_removal_leaves_no_row_anywhere_that_names_the_profile() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    // A type that keeps a home, so that `profile_homes` has a row for the walk
    // to find: a Claude Profile's account is the pair in its own row, and a
    // fixture made of one would leave that table empty.
    let profile = create_profile(&pool, &facts_for("work", codex("work")))
        .await
        .unwrap()
        .unwrap();

    set_grilling_pairing(&pool, id, profile.id, Some(MODEL))
        .await
        .unwrap();
    set_implementation_pairing(&pool, id, profile.id, Some(MODEL))
        .await
        .unwrap();
    set_review_pairing(&pool, id, profile.id, Some(MODEL))
        .await
        .unwrap();

    // The Repo's memory of what it was last grilled with, which is the one
    // table naming a Profile that is not a Conversation's own column.
    start_grilling(
        &pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        Path::new("/var/lib/verkstead/worktrees/verkstead-amber-kestrel"),
        &[],
    )
    .await
    .unwrap();

    let naming = names_a_profile(&pool).await;

    assert!(
        !naming.is_empty(),
        "the schema has tables that name a Profile",
    );

    for (table, column) in &naming {
        assert!(
            naming_rows(&pool, table, column, profile.id).await > 0,
            "nothing in {table}.{column} names the Profile, so its removal is \
             something this test never sees happen",
        );
    }

    assert_eq!(
        delete_profile(&pool, profile.id).await.unwrap(),
        Deleting::Deleted
    );

    for (table, column) in &naming {
        assert_eq!(
            naming_rows(&pool, table, column, profile.id).await,
            0,
            "{table}.{column} still names the Profile that was removed",
        );
    }
}

/// Every table-and-column pair the schema says points at `profiles`, asked of
/// SQLite rather than written down here.
async fn names_a_profile(pool: &SqlitePool) -> Vec<(String, String)> {
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(pool)
    .await
    .unwrap();

    let mut naming = Vec::new();

    for (table,) in tables {
        let keys: Vec<(String, String)> = sqlx::query_as(&format!(
            "SELECT \"table\", \"from\" FROM pragma_foreign_key_list('{table}')"
        ))
        .fetch_all(pool)
        .await
        .unwrap();

        for (points_at, column) in keys {
            if points_at == "profiles" {
                naming.push((table.clone(), column));
            }
        }
    }

    naming.sort();
    naming
}

/// How many rows of one of them name this Profile.
async fn naming_rows(pool: &SqlitePool, table: &str, column: &str, id: i64) -> i64 {
    let (rows,): (i64,) =
        sqlx::query_as(&format!("SELECT COUNT(*) FROM {table} WHERE {column} = ?"))
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();

    rows
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
        half.grilling_pairing.pairing().map(|p| p.profile.id),
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
        both.grilling_pairing
            .pairing()
            .map(|p| p.profile.name.clone()),
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

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    let pairing = conversation
        .grilling_pairing
        .pairing()
        .expect("a Pairing was picked");

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
            .pairing()
            .and_then(|pairing| pairing.model.clone()),
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

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();
    let pairing = conversation
        .grilling_pairing
        .pairing()
        .expect("a Pairing was picked");

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
        conversation
            .grilling_pairing
            .pairing()
            .map(|p| p.profile.name.clone()),
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
        conversation
            .grilling_pairing
            .pairing()
            .map(|p| p.profile.id),
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
    assert_eq!(conversation.grilling_pairing, Picked::Nothing);
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
    assert_eq!(conversation.grilling_pairing, Picked::Nothing);
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
        conversation
            .grilling_pairing
            .pairing()
            .map(|p| p.profile.name.clone()),
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

/// *No grilling* is a choice too, stored apart from having chosen nothing —
/// exactly as *no review* is, one role along.
#[tokio::test]
async fn no_grilling_is_a_choice_rather_than_an_empty_picker() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .grilling_pairing,
        Picked::Nothing,
        "nothing has been picked yet",
    );

    assert_eq!(skip_grilling(&pool, id).await.unwrap(), Chosen::Chosen);

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .grilling_pairing,
        Picked::Skipped,
        "and now something has",
    );
}

/// The two rows are rows of one list here as well, so picking either unpicks
/// the other.
#[tokio::test]
async fn picking_a_grilling_pairing_and_picking_none_replace_each_other() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let fable = saved(&pool, "fable").await;

    set_grilling_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();
    skip_grilling(&pool, id).await.unwrap();

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .grilling_pairing,
        Picked::Skipped,
        "the account it was going to be interviewed by is off it",
    );

    set_grilling_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();

    let picked = load_conversation(&pool, id)
        .await
        .unwrap()
        .unwrap()
        .grilling_pairing;

    assert_eq!(
        picked.pairing().map(|pairing| pairing.profile.id),
        Some(fable.id),
        "and picking an account back takes the row that ran nothing away",
    );
}

/// And it is fixed the moment the work starts, which for a Conversation that
/// will not be grilled is [`start_building`] rather than [`start_grilling`].
#[tokio::test]
async fn no_grilling_cannot_be_picked_once_the_work_has_started() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let fable = saved(&pool, "fable").await;

    skip_grilling(&pool, id).await.unwrap();
    set_implementation_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();
    set_review_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();

    start_building(
        &pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        Path::new("/var/lib/verkstead/worktrees/verkstead-amber-kestrel"),
        &[],
    )
    .await
    .unwrap();

    assert_eq!(
        set_grilling_pairing(&pool, id, fable.id, Some(MODEL))
            .await
            .unwrap(),
        Chosen::NotDrafting,
        "there is no picking left once the work has started",
    );

    assert_eq!(
        load_conversation(&pool, id)
            .await
            .unwrap()
            .unwrap()
            .grilling_pairing,
        Picked::Skipped,
        "and what it started under is exactly where it was",
    );
}

/// What the press that skips the grilling leaves behind: a Conversation
/// Implementing, on the branch and the base commit a grill start would have
/// given it, with the Direction a Brief taken straight to the work is.
#[tokio::test]
async fn starting_without_a_grilling_lands_the_conversation_implementing_inline() {
    let (_dir, pool) = fresh_pool().await;
    let id = conversation(&pool).await;
    let fable = saved(&pool, "fable").await;

    skip_grilling(&pool, id).await.unwrap();
    set_implementation_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();
    set_review_pairing(&pool, id, fable.id, Some(MODEL))
        .await
        .unwrap();

    start_building(
        &pool,
        id,
        "6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7",
        Path::new("/var/lib/verkstead/worktrees/verkstead-amber-kestrel"),
        &[],
    )
    .await
    .unwrap();

    let conversation = load_conversation(&pool, id).await.unwrap().unwrap();

    assert_eq!(conversation.state, Lifecycle::Implementing);
    assert_eq!(
        conversation.direction,
        Some(Direction::Inline),
        "a Brief taken straight to the work is an inline implementation",
    );
    assert_eq!(
        conversation.base_commit.as_deref(),
        Some("6f32b11a0c4d1e8f5b3a97c2d0e4f6a8b1c3d5e7"),
        "and everything the press did against git is recorded as it always is",
    );
    assert_eq!(
        conversation.worktree.as_deref(),
        Some(Path::new(
            "/var/lib/verkstead/worktrees/verkstead-amber-kestrel"
        )),
    );
}
