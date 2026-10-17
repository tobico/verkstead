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
//! **No token crosses on the list.** What a row says of one is whether the home
//! device holds it. The token itself is fetched on a member's own call — see
//! [`crate::peer::authors`] — and a **copy** written into this device's own
//! `secrets.yaml` under the uuid as the rows are written down, so that a Repo
//! here pointed at a mirror commits and pushes from it whether or not its home
//! is answering at the moment of need. It is taken away when the mirror goes,
//! when its device is unlinked, and when the home stops holding one; a home not
//! answering leaves the copy as it stands, and it is used (ADR-0025).

use std::collections::BTreeMap;

use anyhow::Result;
use verkstead_render::{AuthorEdit, AuthorEdited, AuthorEntry, AuthorRemoved, AuthorToken};
use verkstead_schema::Nudge;

use super::{MOST_AN_OUTCOME_IS, Pressed};
use crate::peer::authors::TOKEN;
use crate::relaying::{Call, Streamed, as_json, asking, read_of, word_from};
use crate::settings::{Author, Mirrored, Settings};
use crate::{AppState, store};

/// What a member's Authors are read at, which is the path this device serves
/// its own at — one router mounted twice.
const AUTHORS: &str = "/api/ui/authors";

/// The most a member's Authors may be before the answer is dropped rather than
/// written down: **the Profiles' megabyte**, for its reason. An Author is a few
/// hundred bytes.
const MOST_THE_AUTHORS_ARE: usize = super::MOST_THE_PROFILES_ARE;

/// The most a member's answer about one token may be: **sixty-four
/// kilobytes**, the login's bound beside it. A GitHub token is under a hundred
/// bytes.
const MOST_A_TOKEN_IS: usize = 64 * 1024;

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

/// Put an edit of an Author to the device it is at home on, or say that it is
/// this device's own.
///
/// **The Profile mirrors' hop, for an Author** — see [`super::edited`]: the
/// press is put to the home device as the ordinary edit of its own Author,
/// addressed by the uuid, which is the same there; what comes back is that
/// device's own word, the token's verification included, asked from there with
/// its own `gh`. A home that did not take it is refused naming the machine and
/// nothing here has moved.
///
/// **And the mirror redraws off the refreshed row** rather than off what was
/// typed: a save that landed is followed by the ordinary read of that member,
/// which writes the copy of a token set this way as any refresh does.
pub(crate) async fn edited(
    state: &AppState,
    id: &str,
    edit: &AuthorEdit,
) -> Result<Pressed<AuthorEdited>> {
    let Some(device) = home_of(state, id).await? else {
        return Ok(Pressed::Here);
    };

    let call = Call {
        method: reqwest::Method::POST,
        onwards: format!("{AUTHORS}/{id}"),
        headers: as_json(),
        body: Streamed::saying(serde_json::to_vec(edit)?),
    };

    Ok(
        match word_from::<AuthorEdited>(state.devices.as_ref(), &device, call, MOST_AN_OUTCOME_IS)
            .await
        {
            Ok(said) => {
                if matches!(said, AuthorEdited::Saved { .. }) {
                    read_member(state, &device).await;
                }

                Pressed::Away(said)
            }
            Err(why) => Pressed::Refused(why),
        },
    )
}

/// And a removal, which is the same hop: it takes the Author off its home
/// device, **checking that device's Repos only** — its own refusal naming its
/// own Repos comes back as it stands. A Repo on this device left pointing at
/// the mirror that goes with it refuses its next Start by name, rather than
/// every member being asked first (ADR-0025).
pub(crate) async fn removed(state: &AppState, id: &str) -> Result<Pressed<AuthorRemoved>> {
    let Some(device) = home_of(state, id).await? else {
        return Ok(Pressed::Here);
    };

    let call = Call {
        method: reqwest::Method::POST,
        onwards: format!("{AUTHORS}/{id}/remove"),
        headers: as_json(),
        body: Streamed::Nothing,
    };

    Ok(
        match word_from::<AuthorRemoved>(state.devices.as_ref(), &device, call, MOST_AN_OUTCOME_IS)
            .await
        {
            Ok(said) => {
                if said == AuthorRemoved::Removed {
                    read_member(state, &device).await;
                }

                Pressed::Away(said)
            }
            Err(why) => Pressed::Refused(why),
        },
    )
}

/// The Device Id of the member the Author `id` is at home on — or `None` for
/// one of this device's own, and for a uuid nobody holds, which falls through
/// to the local half and is refused there as an Author that has gone.
///
/// **A uuid `config.yaml` holds is this device's own** whatever a mirror row
/// says, which is the one list's rule.
async fn home_of(state: &AppState, id: &str) -> Result<Option<String>> {
    if state.settings.config().author(id).is_some() {
        return Ok(None);
    }

    Ok(store::author_mirrors(&state.pool)
        .await?
        .into_iter()
        .find(|mirror| mirror.id == id)
        .map(|mirror| mirror.device))
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

    // What the last run wrote down first, so that a Repo pointed at a mirror
    // resolves from the moment the server is up rather than from the moment
    // every member has answered.
    hold(&state).await;

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

            copying(state, gone.into_iter().map(|id| (id, Copy::Gone)).collect()).await;
            hold(state).await;
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

    // And once more whether or not any row moved, for the device names the
    // mirrors are held under: the membership moving is also a member renamed.
    hold(state).await;
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

    // What this device held of it before, for the copies of the Authors that
    // have gone from home since.
    let before = match store::author_mirrors(&state.pool).await {
        Ok(rows) => rows
            .into_iter()
            .filter(|row| row.device == device)
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        Err(error) => {
            tracing::error!(
                error = ?error,
                device,
                "the Authors held of a member could not be read, so its mirrors stand",
            );
            return;
        }
    };

    let moved = match store::record_author_mirrors(&state.pool, device, &at_home).await {
        Ok(moved) => moved,
        Err(error) => {
            tracing::error!(
                error = ?error,
                device,
                "a member's Authors could not be written down",
            );
            return;
        }
    };

    let mut copies: BTreeMap<String, Copy> =
        before.into_iter().map(|id| (id, Copy::Gone)).collect();

    for row in &at_home {
        let copy = match row.holds_token {
            false => Copy::Gone,
            true => token_of(state, device, &row.id).await,
        };

        copies.insert(row.id.clone(), copy);
    }

    let copied = copying(state, copies).await;

    if moved || copied {
        hold(state).await;
        state.nudges.announce_here(Nudge::Authors);
    }
}

/// What becomes of the copy of one mirror's token.
#[derive(Debug)]
enum Copy {
    /// Written under the uuid, over whatever copy was there.
    Held(String),

    /// Taken away: the mirror has gone, or its home holds no token for it.
    Gone,

    /// Left as it stands: the home did not say.
    AsItStands,
}

/// What the member at `device` says the token of its Author `id` is.
///
/// **A home not answering leaves the copy as it stands**, and so does any
/// answer this device cannot read — a refusal of a uuid included, which is an
/// Author removed between the list and this call and goes with its row on the
/// next read. Only a home that answers *none* takes the copy away.
async fn token_of(state: &AppState, device: &str, id: &str) -> Copy {
    match token_at(state.devices.as_ref(), device, id).await {
        Ok(Some(token)) => Copy::Held(token),
        Ok(None) => Copy::Gone,
        Err(why) => {
            tracing::debug!(
                device,
                author = id,
                "a member's Author's token could not be read, so the copy stands as it is: {}",
                why.saying,
            );
            Copy::AsItStands
        }
    }
}

/// The token the member at `device` holds for its own Author `id`, `None`
/// where it holds none — or the refusal naming the machine, which is a home
/// not answering, a caller it holds no membership for, or a uuid that is not
/// one of its own Authors (ADR-0025).
///
/// Public for the suite that stands two devices up and asks it, as the
/// refresher here does.
pub async fn token_at(
    devices: Option<&crate::device::Devices>,
    device: &str,
    id: &str,
) -> Result<Option<String>, crate::Refusal> {
    let said: AuthorToken = word_from(
        devices,
        device,
        asking(TOKEN.replace("{author}", id)),
        MOST_A_TOKEN_IS,
    )
    .await?;

    Ok(said.token)
}

/// Write `copies` into this device's `secrets.yaml`, each under its uuid, and
/// every other secret in the file left as it was — and say whether the file
/// moved.
///
/// **One read and one write**, with nothing awaited between them: a save on
/// the settings page writes the whole file too, and the window in which one
/// could undo the other is kept to the two calls.
///
/// **A uuid `config.yaml` holds is never touched**: that is this device's own
/// Author, and the token under it is its own rather than a copy of anybody's.
async fn copying(state: &AppState, copies: BTreeMap<String, Copy>) -> bool {
    let settings = state.settings.clone();

    let written = tokio::task::spawn_blocking(move || {
        let config = settings.config();
        let mut secrets = settings.secrets();
        let mut moved = false;

        for (id, copy) in copies {
            if config.author(&id).is_some() {
                continue;
            }

            let now = secrets.author_token(&id).map(str::to_owned);

            secrets = match copy {
                Copy::Held(token) if now.as_deref() != Some(&token) => {
                    moved = true;
                    secrets.with_author_token(&id, Some(token))
                }
                Copy::Gone if now.is_some() => {
                    moved = true;
                    secrets.with_author_token(&id, None)
                }
                _ => secrets,
            };
        }

        if !moved {
            return Ok(false);
        }

        settings.save_secrets(&secrets).map(|()| true)
    })
    .await;

    match written {
        Ok(Ok(moved)) => moved,
        Ok(Err(error)) => {
            tracing::error!(
                error = ?error,
                "the copies of the members' Authors' tokens could not be written into \
                 secrets.yaml, so the ones there stand",
            );
            false
        }
        Err(error) => {
            tracing::error!(
                error = ?error,
                "writing the copies of the members' Authors' tokens ended badly",
            );
            false
        }
    }
}

/// Hand every mirror this device has written down to the settings, which is
/// where a Repo's Author is resolved — see [`Settings::holding_mirrors`] — each
/// with the name of the device it is at home on, for the refusal that names it.
///
/// **A store that cannot be read leaves what is held as it is**, for the reason
/// a member not answering keeps its rows.
async fn hold(state: &AppState) {
    match mirrored(state).await {
        Ok(mirrors) => state.settings.holding_mirrors(mirrors),
        Err(error) => tracing::error!(
            error = ?error,
            "the Author mirrors could not be read, so the ones held stand",
        ),
    }
}

/// Every mirror row, as the settings hold one.
async fn mirrored(state: &AppState) -> Result<Vec<Mirrored>> {
    let mirrors = store::author_mirrors(&state.pool).await?;

    if mirrors.is_empty() {
        return Ok(Vec::new());
    }

    let homes = crate::profiles::members_by_device(&state.pool).await?;

    Ok(mirrors
        .into_iter()
        .map(|mirror| {
            let device = crate::profiles::whose(&homes, &mirror.device).name;

            Mirrored::of(
                Author::of(
                    mirror.id,
                    Some(mirror.display_name),
                    Some(mirror.name),
                    Some(mirror.email),
                ),
                device,
                mirror.holds_token,
            )
        })
        .collect())
}
