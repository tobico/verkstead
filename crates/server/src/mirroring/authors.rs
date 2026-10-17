//! The **Author mirrors**: every member's Authors written down here beside this
//! device's own, so that every device of a cluster draws one list (ADR-0025,
//! borrowing ADR-0020's *Shared Profiles*).
//!
//! **The Profile mirrors' shape, kept to what an Author is.** A member answers
//! its Authors on `GET /api/ui/authors`, which the viewer namespace already
//! serves it over the Peer Listener behind the Member Gate, so there is no new
//! peer route; this device asks at the start, whenever a member says its
//! Authors moved — [`Nudge::Authors`], or [`Nudge::Everything`] when a stream to
//! it is taken up — and whenever the membership moves. A member not answering
//! keeps the rows it last gave; an Author removed at home is gone from what that
//! device answers, so its row goes on the next read; a device unlinked takes its
//! rows with it.
//!
//! **Only a member's own rows are taken**, the ones it answers with no device
//! mark, so a mirror is one hop from its home and never a mirror of a mirror —
//! the Profile mirrors' rule, for its reason.
//!
//! **Keyed by the uuid**, which is the same everywhere: unlike a Profile there
//! is no local id, because what names an Author — a Repo's record — already
//! names it by the uuid. See [`store::record_author_mirrors`].
//!
//! **No token crosses here.** What a row says of one is whether the home device
//! holds it.

use anyhow::Result;
use verkstead_render::AuthorEntry;
use verkstead_schema::Nudge;

use crate::relaying::read_of;
use crate::settings::Settings;
use crate::{AppState, store};

/// What a member's Authors are read at, which is the path this device serves
/// its own at — one router mounted twice.
const AUTHORS: &str = "/api/ui/authors";

/// The most a member's Authors may be before the answer is dropped rather than
/// written down: **the Profiles' megabyte**, for its reason. An Author is a few
/// hundred bytes.
const MOST_THE_AUTHORS_ARE: usize = super::MOST_THE_PROFILES_ARE;

/// The one list: this device's own Authors in the order `config.yaml` holds
/// them, with no device mark, and then every member's, each marked with the
/// device it is at home on.
///
/// **A mirror of a uuid this device holds itself is left off**, which is the
/// one list rather than two rows for one Author: whatever this device's own
/// files say about it is what it is here.
pub(crate) async fn listed(state: &AppState) -> Result<Vec<AuthorEntry>> {
    let mut listed = own(&state.settings);

    let mirrors = store::author_mirrors(&state.pool).await?;

    if mirrors.is_empty() {
        return Ok(listed);
    }

    let homes = crate::profiles::members_by_device(&state.pool).await?;

    for mirror in mirrors {
        if listed.iter().any(|own| own.id == mirror.id) {
            continue;
        }

        listed.push(AuthorEntry {
            device: Some(crate::profiles::whose(&homes, &mirror.device)),
            id: mirror.id,
            display_name: mirror.display_name,
            name: mirror.name,
            email: mirror.email,
            holds_token: mirror.holds_token,
        });
    }

    Ok(listed)
}

/// This device's own Authors, as the one list draws them: off the two files at
/// the moment they are asked for, and nothing of the token but whether there
/// is one.
pub(crate) fn own(settings: &Settings) -> Vec<AuthorEntry> {
    let secrets = settings.secrets();

    settings
        .config()
        .authors()
        .iter()
        .map(|author| AuthorEntry {
            id: author.id().to_owned(),
            display_name: author.display_name().unwrap_or_default().to_owned(),
            name: author.name().unwrap_or_default().to_owned(),
            email: author.email().unwrap_or_default().to_owned(),
            holds_token: secrets.author_token(author.id()).is_some(),
            device: None,
        })
        .collect()
}

/// Keep every member's Author mirrors fresh, for as long as this server runs.
///
/// Spawned beside the Profile mirrors' refresher and for its reason. Nothing at
/// all on a router with no identity, which is linked to nothing.
pub(crate) fn refreshing(state: &AppState) {
    if state.devices.is_none() {
        return;
    }

    let state = state.clone();

    tokio::spawn(async move { held(state).await });
}

/// Every member read once, and then whatever a Nudge says to read again —
/// subscribed before the first round, so a Nudge landing during it is heard.
async fn held(state: AppState) {
    let mut moved = state.nudges.subscribe();

    read_every_member(&state).await;

    loop {
        match moved.recv().await {
            Ok(moved) => match (moved.device, &moved.moved) {
                (Some(device), Nudge::Authors | Nudge::Everything) => {
                    read_member(&state, &device).await
                }

                (None, Nudge::Devices) => read_every_member(&state).await,

                _ => {}
            },

            Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                tracing::debug!(
                    missed,
                    "the Author mirrors fell behind the Nudges, so every member's Authors are \
                     read again",
                );
                read_every_member(&state).await;
            }

            Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
        }
    }
}

/// Read every member's Authors, one after another — and take away the mirrors
/// of every device that is not one any more.
async fn read_every_member(state: &AppState) {
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let members = match devices.membership().rows().await {
        Ok(members) => members,
        Err(why) => {
            tracing::error!(
                error = ?why,
                "the devices this one is linked to could not be read, so no member's Authors \
                 were read and the mirrors stand as they are",
            );
            return;
        }
    };

    match store::forget_author_mirrors_of_departed(&state.pool).await {
        Ok(gone) if gone.is_empty() => {}

        Ok(gone) => {
            tracing::info!(
                authors = gone.len(),
                "the Authors of devices this one is no longer linked to are gone",
            );

            state.nudges.announce_here(Nudge::Authors);
        }

        Err(error) => tracing::error!(
            error = ?error,
            "the Authors of devices this one is no longer linked to could not be taken away",
        ),
    }

    for member in members {
        read_member(state, &member.device).await;
    }
}

/// Read one member's Authors and write down what it said, whole.
///
/// **A read that could not be made leaves every row as it is**, and so does an
/// answer this device cannot read: it has no better account of that machine's
/// Authors than the one it already wrote down.
///
/// **And the news stays here**: what moved is the rows this device holds of
/// somebody else's, and a member told it would read it as *this device's
/// Authors moved* and come back, round for ever.
async fn read_member(state: &AppState, device: &str) {
    let Some(devices) = state.devices.as_ref() else {
        return;
    };

    let Some(said) = read_of(
        devices,
        device,
        AUTHORS.to_owned(),
        "Authors",
        MOST_THE_AUTHORS_ARE,
    )
    .await
    else {
        return;
    };

    let rows = match serde_json::from_slice::<Vec<AuthorEntry>>(&said) {
        Ok(rows) => rows,
        Err(why) => {
            tracing::warn!(
                device,
                "a member answered its Authors in a way this device cannot read, so the mirrors \
                 of it stand: {why}",
            );
            return;
        }
    };

    // Its own rows and nothing it is itself mirroring.
    let at_home: Vec<store::AuthorMirror> = rows
        .into_iter()
        .filter(|row| row.device.is_none())
        .map(|row| store::AuthorMirror {
            id: row.id,
            device: device.to_owned(),
            display_name: row.display_name,
            name: row.name,
            email: row.email,
            holds_token: row.holds_token,
        })
        .collect();

    match store::record_author_mirrors(&state.pool, device, &at_home).await {
        Ok(true) => state.nudges.announce_here(Nudge::Authors),
        Ok(false) => {}
        Err(error) => tracing::error!(
            error = ?error,
            device,
            "a member's Authors could not be written down",
        ),
    }
}
