//! Managing Agent Profiles: everything between a workbench form and a row in
//! the store, and the check that says whether a saved one can still be run
//! under.
//!
//! The account is judged against the filesystem and against the Watched Paths,
//! and both are the server's: the boundary is a security boundary and a form
//! that checked it would be a courtesy, since this endpoint is reachable without
//! one. The same boundary governs a Profile's account as governs a Repo's path,
//! because it is one rule about what Verkstead may touch and not one rule per
//! feature. What an account *is* — Claude's pair, and the single home each
//! backend after it keeps one under — is the agent type's, and every judgement
//! here is made per type for that reason.
//!
//! Nothing here mounts anything. A Profile is a record of an account a session
//! will later be run under; the bind-mounting arrives with the stage that runs
//! one.

use std::path::Path;

use anyhow::Result;
use sqlx::SqlitePool;
use verkstead_render::{
    Broken, PairingView, PickedView, ProfileAccount, ProfileChoice, ProfileChosen, ProfileDeleted,
    ProfileEdit, ProfileEntry, ProfileSaved, RoleChoice,
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

/// Every Profile, each with whether its account is still where it was left.
pub(crate) async fn listed(pool: &SqlitePool, watched: &WatchedPaths) -> Result<Vec<ProfileEntry>> {
    entries(watched, store::profiles(pool).await?).await
}

/// The same reading for one Pairing, for the panes that show a Conversation's
/// own choices rather than the whole list.
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

/// And the same reading for a whole choice, for the one role that can be picked
/// away as well as paired.
///
/// The three states come across unchanged: a Pairing is read as one, the row
/// that runs no session stays what it is, and a Profile whose row has gone
/// reads as nothing picked — which is what it is, since there is no account
/// left to launch under.
pub(crate) async fn picked(watched: &WatchedPaths, picked: store::Picked) -> Result<PickedView> {
    Ok(match picked {
        store::Picked::Nothing => PickedView::Nothing,
        store::Picked::Skipped => PickedView::Skipped,
        store::Picked::Under(under) => match pairing(watched, Some(under)).await? {
            Some(pairing) => PickedView::Under(pairing),
            None => PickedView::Nothing,
        },
    })
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
                account: account(&profile.account),
                models: profile.models,
            })
            .collect()
    })
    .await?)
}

/// Whether everything the next stage needs before it will start is settled.
///
/// Every Pairing complete — a Profile *and* a model — and no Profile broken.
/// Broken as well as chosen because a pair that has gone is a session that
/// fails to start, and the whole reason this is answered here is to say so
/// while the human is looking.
///
/// A Profile chosen before pairings existed has no model and so is not a
/// Pairing: while a Conversation is drafting that is a choice to make again,
/// which is what this says. Nothing asks it of a Conversation past drafting —
/// there is no start left to be ready for — so the carried model this would
/// refuse is never the one that runs anything.
///
/// **A role picked away is settled**, and settles this: there is no session to
/// fail to start, so a Conversation that will not be grilled, or will not be
/// reviewed, is as ready as one that will. What it is not is an empty picker —
/// see [`verkstead_render::PickedView`].
pub(crate) fn ready_to_grill(
    grilling: &PickedView,
    implementation: Option<&PairingView>,
    review: &PickedView,
) -> bool {
    runnable(implementation) && [grilling, review].into_iter().all(settled)
}

/// Whether one role that can be picked away is settled: a Pairing something
/// could be launched under, or the row that launches nothing.
fn settled(picked: &PickedView) -> bool {
    match picked {
        PickedView::Nothing => false,
        PickedView::Skipped => true,
        PickedView::Under(pairing) => runnable(Some(pairing)),
    }
}

/// Whether one picked Pairing is something a session could be launched under.
fn runnable(pairing: Option<&PairingView>) -> bool {
    pairing.is_some_and(|pairing| pairing.model.is_some() && pairing.profile.broken.is_none())
}

/// Why this Profile cannot be run under as things stand, or `None` while its
/// account is where it was left.
///
/// Asked of the boundary and not only of the filesystem: a directory replaced by
/// a symlink pointing out of the Watched Paths still exists, and mounting it
/// would be reaching around the boundary with a path that was admitted once.
///
/// Each of an account's paths is named by what it is, for the reason the
/// refusals are: a Profile whose config file has gone and one whose directory
/// has gone are two different things to go and put right.
fn broken(watched: &WatchedPaths, profile: &store::Profile) -> Option<Broken> {
    let paths: Vec<(&std::path::PathBuf, Broken)> = match &profile.account {
        store::Account::Claude {
            claude_dir,
            config_file,
        } => vec![
            (claude_dir, Broken::DirMissing),
            (config_file, Broken::ConfigMissing),
        ],
        store::Account::Codex { home } => vec![(home, Broken::HomeMissing)],
    };

    for (path, missing) in paths {
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
    let account = edit.account.clone();

    Ok(
        tokio::task::spawn_blocking(move || inspect(&watched, &account))
            .await?
            .map(|account| store::ProfileFacts {
                name,
                account,
                models,
            }),
    )
}

/// The account, resolved — or the reason it is refused.
///
/// One arm per agent type, because what a Profile names is that type's own
/// shape: Claude's pair here, and the single home each backend after it keeps
/// its whole account under when its stage lands.
///
/// Every path is judged the same way and refused by its own name: pointing the
/// config field at the directory is an easy mistake, and "that path is wrong"
/// would not say which one.
fn inspect(
    watched: &WatchedPaths,
    account: &ProfileAccount,
) -> Result<store::Account, ProfileSaved> {
    match account {
        ProfileAccount::Claude {
            claude_dir,
            config_file,
        } => {
            let dir = match watched.admit(Path::new(claude_dir.trim())) {
                Admission::Inside(path) => path,
                Admission::NotAbsolute => return Err(ProfileSaved::DirNotAbsolute),
                Admission::Missing => return Err(ProfileSaved::DirMissing),
                Admission::Outside => return Err(ProfileSaved::DirOutsideWatchedPaths),
            };

            if !dir.is_dir() {
                return Err(ProfileSaved::NotADirectory);
            }

            let config = match watched.admit(Path::new(config_file.trim())) {
                Admission::Inside(path) => path,
                Admission::NotAbsolute => return Err(ProfileSaved::ConfigNotAbsolute),
                Admission::Missing => return Err(ProfileSaved::ConfigMissing),
                Admission::Outside => return Err(ProfileSaved::ConfigOutsideWatchedPaths),
            };

            if !config.is_file() {
                return Err(ProfileSaved::NotAFile);
            }

            Ok(store::Account::Claude {
                claude_dir: dir,
                config_file: config,
            })
        }

        ProfileAccount::Codex { home } => {
            let home = match watched.admit(Path::new(home.trim())) {
                Admission::Inside(path) => path,
                Admission::NotAbsolute => return Err(ProfileSaved::HomeNotAbsolute),
                Admission::Missing => return Err(ProfileSaved::HomeMissing),
                Admission::Outside => return Err(ProfileSaved::HomeOutsideWatchedPaths),
            };

            if !home.is_dir() {
                return Err(ProfileSaved::HomeNotADirectory);
            }

            Ok(store::Account::Codex { home })
        }
    }
}

/// Choose the Pairing a Conversation's grilling session will run under — or the
/// row that says there is to be no grilling at all.
///
/// One press either way, for the reason the review's is one: the picker offers
/// them as one list, and nothing is judged about a role that runs nothing, there
/// being no Profile to have gone and no model to have been retyped.
pub(crate) async fn choose_grilling(
    pool: &SqlitePool,
    id: i64,
    choice: &RoleChoice,
) -> Result<ProfileChosen> {
    let Some(choice) = &choice.pairing else {
        return Ok(chosen(store::skip_grilling(pool, id).await?));
    };

    if let Some(refusal) = unlisted(pool, choice).await? {
        return Ok(refused(refusal));
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
        return Ok(refused(refusal));
    }

    Ok(chosen(
        store::set_implementation_pairing(pool, id, choice.profile_id, Some(&choice.model)).await?,
    ))
}

/// And the one the wrap-up's review session will run under — or the row that
/// says there is to be no review session at all.
///
/// One press either way, because the picker offers them as one list: nothing is
/// judged about a review that runs nothing, there being no Profile to have gone
/// and no model to have been retyped.
pub(crate) async fn choose_review(
    pool: &SqlitePool,
    id: i64,
    choice: &RoleChoice,
) -> Result<ProfileChosen> {
    let Some(choice) = &choice.pairing else {
        return Ok(chosen(store::skip_review(pool, id).await?));
    };

    if let Some(refusal) = unlisted(pool, choice).await? {
        return Ok(refused(refusal));
    }

    Ok(chosen(
        store::set_review_pairing(pool, id, choice.profile_id, Some(&choice.model)).await?,
    ))
}

/// What is wrong with a picked Pairing, where anything is.
///
/// Its own two words rather than one of the outcome types around it, because two
/// presses ask this and each answers in a vocabulary of its own: the drafting
/// pickers answer as [`ProfileChosen`], and a steer answers as
/// [`verkstead_render::ConversationSteered`]. The question is the same either
/// way — is this still a Profile, and does it still list that model?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unlisted {
    /// There is no Profile with that id: it was removed between the list the
    /// page read and the pick it made from it.
    NoSuchProfile,

    /// That Profile does not list that model, for the same reason.
    NoSuchModel,
}

/// Why this is not a Pairing to record, or `None` where the Profile lists the
/// model it was picked with.
///
/// Asked here rather than in the store, because it is a question about a Profile
/// read as a row: a Profile written before the model list existed carries its
/// one model in the old column, and a statement that checked `profile_models`
/// alone would refuse the only model that Profile has.
pub(crate) async fn unlisted(
    pool: &SqlitePool,
    choice: &ProfileChoice,
) -> Result<Option<Unlisted>> {
    let Some(profile) = store::load_profile(pool, choice.profile_id).await? else {
        return Ok(Some(Unlisted::NoSuchProfile));
    };

    Ok((!profile.models.contains(&choice.model)).then_some(Unlisted::NoSuchModel))
}

/// What is wrong with a pick, as the drafting pickers say it.
fn refused(unlisted: Unlisted) -> ProfileChosen {
    match unlisted {
        Unlisted::NoSuchProfile => ProfileChosen::NoSuchProfile,
        Unlisted::NoSuchModel => ProfileChosen::NoSuchModel,
    }
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

/// The store's account as the viewer receives it, held to the wire's the same
/// way: one arm per agent type, and the paths that type has.
///
/// The paths are stored as UTF-8 in the first place — one that is not cannot be
/// saved — so nothing is lost putting them back on the wire.
fn account(account: &store::Account) -> ProfileAccount {
    match account {
        store::Account::Claude {
            claude_dir,
            config_file,
        } => ProfileAccount::Claude {
            claude_dir: claude_dir.to_string_lossy().into_owned(),
            config_file: config_file.to_string_lossy().into_owned(),
        },
        store::Account::Codex { home } => ProfileAccount::Codex {
            home: home.to_string_lossy().into_owned(),
        },
    }
}
