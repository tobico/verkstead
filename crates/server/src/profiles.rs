//! Managing Agent Profiles: everything between a workbench form and a row in
//! the store, and the check that says whether a saved one can still be run
//! under.
//!
//! The pair is judged against the filesystem and against the Watched Paths, and
//! both are the server's: the boundary is a security boundary and a form that
//! checked it would be a courtesy, since this endpoint is reachable without one.
//! The same boundary governs a Profile's pair as governs a Repo's path, because
//! it is one rule about what Verkstead may touch and not one rule per feature.
//!
//! Nothing here mounts anything. A Profile is a record of an account a session
//! will later be run under; the bind-mounting arrives with the stage that runs
//! one.

use std::path::{Path, PathBuf};

use anyhow::Result;
use sqlx::SqlitePool;
use verkstead_render::{
    Broken, PairingView, ProfileChoice, ProfileChosen, ProfileDeleted, ProfileEdit, ProfileEntry,
    ProfileSaved,
};

use crate::store;
use crate::watched::{Admission, WatchedPaths};

/// Save a new Profile, or say why not.
pub(crate) async fn create(
    pool: &SqlitePool,
    watched: &WatchedPaths,
    edit: &ProfileEdit,
) -> Result<ProfileSaved> {
    let facts = match checked(watched, edit).await? {
        Ok(facts) => facts,
        Err(refusal) => return Ok(refusal),
    };

    Ok(match store::create_profile(pool, &facts).await? {
        Some(_) => ProfileSaved::Saved,
        None => ProfileSaved::NameTaken,
    })
}

/// Rewrite one, whole. Everything about a Profile is the human's to change:
/// nothing has been built from it that a change would contradict.
pub(crate) async fn edit(
    pool: &SqlitePool,
    watched: &WatchedPaths,
    id: i64,
    edit: &ProfileEdit,
) -> Result<ProfileSaved> {
    let facts = match checked(watched, edit).await? {
        Ok(facts) => facts,
        Err(refusal) => return Ok(refusal),
    };

    Ok(match store::update_profile(pool, id, &facts).await? {
        store::Saving::Saved => ProfileSaved::Saved,
        store::Saving::NoSuchProfile => ProfileSaved::NoSuchProfile,
        store::Saving::NameTaken => ProfileSaved::NameTaken,
    })
}

/// Remove one nobody is running under.
pub(crate) async fn remove(pool: &SqlitePool, id: i64) -> Result<ProfileDeleted> {
    Ok(match store::delete_profile(pool, id).await? {
        store::Deleting::Deleted => ProfileDeleted::Removed,
        store::Deleting::NoSuchProfile => ProfileDeleted::NoSuchProfile,
        store::Deleting::InUse => ProfileDeleted::InUse,
    })
}

/// Every Profile, each with whether its pair is still where it was left.
pub(crate) async fn listed(pool: &SqlitePool, watched: &WatchedPaths) -> Result<Vec<ProfileEntry>> {
    entries(watched, store::profiles(pool).await?).await
}

/// The same reading for one Pairing, for the panes that show a Conversation's
/// two choices rather than the whole list.
pub(crate) async fn pairing(
    watched: &WatchedPaths,
    pairing: Option<store::Pairing>,
) -> Result<Option<PairingView>> {
    let Some(pairing) = pairing else {
        return Ok(None);
    };

    let model = pairing.model;

    Ok(entries(watched, vec![pairing.profile])
        .await?
        .pop()
        .map(|profile| PairingView { profile, model }))
}

/// Read a batch of Profiles into rows, looking at the filesystem once for the
/// lot of them.
///
/// Off the runtime, because every one of those looks is a blocking read — and
/// together rather than one call per Profile, since the whole point of the batch
/// is that it is one hop off the runtime instead of a list's worth.
async fn entries(
    watched: &WatchedPaths,
    profiles: Vec<store::Profile>,
) -> Result<Vec<ProfileEntry>> {
    let watched = watched.clone();

    Ok(tokio::task::spawn_blocking(move || {
        profiles
            .into_iter()
            .map(|profile| ProfileEntry {
                id: profile.id,
                broken: broken(&watched, &profile),
                name: profile.name,
                // Stored as UTF-8 in the first place — a path that is not cannot
                // be saved — so nothing is lost putting them back on the wire.
                claude_dir: profile.claude_dir.to_string_lossy().into_owned(),
                config_file: profile.config_file.to_string_lossy().into_owned(),
                models: profile.models,
                agent_type: agent_type(profile.agent_type),
            })
            .collect()
    })
    .await?)
}

/// Whether everything the next stage needs before it will grill is settled.
///
/// Both Pairings complete — a Profile *and* a model — and neither Profile
/// broken. Broken as well as chosen because a pair that has gone is a session
/// that fails to start, and the whole reason this is answered here is to say so
/// while the human is looking.
///
/// A Profile chosen before pairings existed has no model and so is not a
/// Pairing: while a Conversation is drafting that is a choice to make again,
/// which is what this says. Nothing asks it of a Conversation past drafting —
/// there is no grilling left to be ready for — so the carried model this would
/// refuse is never the one that runs anything.
pub(crate) fn ready_to_grill(
    grilling: Option<&PairingView>,
    implementation: Option<&PairingView>,
) -> bool {
    [grilling, implementation].into_iter().all(|chosen| {
        chosen.is_some_and(|pairing| pairing.model.is_some() && pairing.profile.broken.is_none())
    })
}

/// Why this Profile cannot be run under as things stand, or `None` while its
/// pair is where it was left.
///
/// Asked of the boundary and not only of the filesystem: a directory replaced by
/// a symlink pointing out of the Watched Paths still exists, and mounting it
/// would be reaching around the boundary with a path that was admitted once.
fn broken(watched: &WatchedPaths, profile: &store::Profile) -> Option<Broken> {
    for (path, missing) in [
        (&profile.claude_dir, Broken::DirMissing),
        (&profile.config_file, Broken::ConfigMissing),
    ] {
        match watched.admit(path) {
            Admission::Inside(_) => {}
            Admission::Missing | Admission::NotAbsolute => return Some(missing),
            Admission::Outside => return Some(Broken::OutsideWatchedPaths),
        }
    }

    None
}

/// What the human typed, checked — or the reason it is not going to be saved.
///
/// The filesystem half runs off the runtime for the reason a registration's
/// does: resolving a path is blocking, and a save is rare enough that the thread
/// it borrows costs nothing.
async fn checked(
    watched: &WatchedPaths,
    edit: &ProfileEdit,
) -> Result<Result<store::ProfileFacts, ProfileSaved>> {
    let name = edit.name.trim().to_owned();

    // Trimmed and the blanks dropped: the form hands these over a line apiece,
    // and a trailing newline is not a model. Their order is kept, because it is
    // the order the human typed and reading the list back should say so.
    let models: Vec<String> = edit
        .models
        .iter()
        .map(|model| model.trim().to_owned())
        .filter(|model| !model.is_empty())
        .collect();

    if name.is_empty() {
        return Ok(Err(ProfileSaved::Nameless));
    }

    if models.is_empty() {
        return Ok(Err(ProfileSaved::Modelless));
    }

    let watched = watched.clone();
    let claude_dir = PathBuf::from(edit.claude_dir.trim());
    let config_file = PathBuf::from(edit.config_file.trim());

    Ok(
        tokio::task::spawn_blocking(move || inspect(&watched, &claude_dir, &config_file))
            .await?
            .map(|(claude_dir, config_file)| store::ProfileFacts {
                name,
                claude_dir,
                config_file,
                models,
                // One type, and the form does not offer a choice of one. What
                // makes the discriminator real is the column, not a picker.
                agent_type: store::AgentType::Claude,
            }),
    )
}

/// The pair, resolved — or the reason it is refused.
///
/// Each half is judged the same way and refused by its own name: pointing the
/// config field at the directory is an easy mistake, and "that path is wrong"
/// would not say which one.
fn inspect(
    watched: &WatchedPaths,
    claude_dir: &Path,
    config_file: &Path,
) -> Result<(PathBuf, PathBuf), ProfileSaved> {
    let dir = match watched.admit(claude_dir) {
        Admission::Inside(path) => path,
        Admission::NotAbsolute => return Err(ProfileSaved::DirNotAbsolute),
        Admission::Missing => return Err(ProfileSaved::DirMissing),
        Admission::Outside => return Err(ProfileSaved::DirOutsideWatchedPaths),
    };

    if !dir.is_dir() {
        return Err(ProfileSaved::NotADirectory);
    }

    let config = match watched.admit(config_file) {
        Admission::Inside(path) => path,
        Admission::NotAbsolute => return Err(ProfileSaved::ConfigNotAbsolute),
        Admission::Missing => return Err(ProfileSaved::ConfigMissing),
        Admission::Outside => return Err(ProfileSaved::ConfigOutsideWatchedPaths),
    };

    if !config.is_file() {
        return Err(ProfileSaved::NotAFile);
    }

    Ok((dir, config))
}

/// Choose the Pairing a Conversation's grilling session will run under.
pub(crate) async fn choose_grilling(
    pool: &SqlitePool,
    id: i64,
    choice: &ProfileChoice,
) -> Result<ProfileChosen> {
    if let Some(refusal) = unlisted(pool, choice).await? {
        return Ok(refusal);
    }

    Ok(chosen(
        store::set_grilling_pairing(pool, id, choice.profile_id, Some(&choice.model)).await?,
    ))
}

/// And the one its implementation will run under.
pub(crate) async fn choose_implementation(
    pool: &SqlitePool,
    id: i64,
    choice: &ProfileChoice,
) -> Result<ProfileChosen> {
    if let Some(refusal) = unlisted(pool, choice).await? {
        return Ok(refusal);
    }

    Ok(chosen(
        store::set_implementation_pairing(pool, id, choice.profile_id, Some(&choice.model)).await?,
    ))
}

/// Why this is not a Pairing to record, or `None` where the Profile lists the
/// model it was picked with.
///
/// Asked here rather than in the store, because it is a question about a Profile
/// read as a row: a Profile written before the model list existed carries its
/// one model in the old column, and a statement that checked `profile_models`
/// alone would refuse the only model that Profile has.
async fn unlisted(pool: &SqlitePool, choice: &ProfileChoice) -> Result<Option<ProfileChosen>> {
    let Some(profile) = store::load_profile(pool, choice.profile_id).await? else {
        return Ok(Some(ProfileChosen::NoSuchProfile));
    };

    Ok((!profile.models.contains(&choice.model)).then_some(ProfileChosen::NoSuchModel))
}

/// The store's outcome as the viewer receives it. One word either side, and this
/// is where the two vocabularies are held to each other.
fn chosen(chosen: store::Chosen) -> ProfileChosen {
    match chosen {
        store::Chosen::Chosen => ProfileChosen::Chosen,
        store::Chosen::NoSuchConversation => ProfileChosen::NoSuchConversation,
        store::Chosen::NoSuchProfile => ProfileChosen::NoSuchProfile,
        store::Chosen::NotDrafting => ProfileChosen::NotDrafting,
    }
}

/// The store's agent type as the viewer receives it, held to the wire's the same
/// way.
fn agent_type(agent_type: store::AgentType) -> verkstead_render::AgentType {
    match agent_type {
        store::AgentType::Claude => verkstead_render::AgentType::Claude,
    }
}
