//! Taking a Conversation in from another device, and letting go of one whose
//! move fell over (ADR-0020, *Transfer*).
//!
//! **The receiving half of a move.** The device holding the work does the
//! deciding — which of this device's Repos is that repository, which of its
//! Agent Profiles each Pairing means — because that is the end that acts on the
//! answers, and [`crate::preflight`] is where it asks them. What is left here is
//! the writing: a row of this device's own, under ids this device issued,
//! carrying the two things that are the cluster's rather than anybody's — the
//! **Rank** and the **birth key**.
//!
//! **And the answer is the commit point of the whole move.** The id this device
//! numbered its copy is what the sending device writes on its own copy as the
//! mark saying where the live record now is; until that answer arrives nothing
//! has changed over there and the work is still being done. Which is what makes
//! a failure safe, and what the sweep below is for: a move that fell over after
//! this wrote and before that mark landed leaves a copy nobody is going to use,
//! and the device that sent it takes it back rather than leaving a half-record
//! on a machine the work never reached.
//!
//! **Gated to members like everything else on this listener.** Writing a
//! Conversation into somebody's database is not a stranger's press:
//! [`super::router`] puts [`super::members_only`] over this, and a caller whose
//! certificate this device holds no membership for never reaches it.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, post};
use verkstead_render::{Arrived, ConversationAcross, PickedAcross, ProfileAcross};

use crate::{AppState, store};

/// Where a member puts a Conversation it is moving onto this device.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace: no browser makes
/// this call and no page draws its answer. The Repos and the harnesses beside it
/// are readings of this machine; this is the one press a member makes on this
/// listener that writes to the record.
pub const TRANSFERS: &str = "/api/peer/v1/transfers";

/// And where it takes one back, the id being the one this device answered with.
pub const ONE_TRANSFER: &str = "/api/peer/v1/transfers/{id}";

/// `POST /api/peer/v1/transfers` — write the arriving Conversation down, and
/// answer with the id it goes by here.
///
/// The Repo is refused where this registry no longer holds it and each Pairing
/// where no Profile here is the one it names — both by name, because both are
/// the sending device acting on a reading that has gone stale and both are worth
/// saying out loud on the Timeline the failure ends up on.
pub(crate) async fn take(
    State(state): State<AppState>,
    Json(arriving): Json<ConversationAcross>,
) -> Response {
    let born = store::Birth {
        device: arriving.born.device.clone(),
        id: arriving.born.id,
    };

    let (grilling, implementation, review) = match resolved(&state, &arriving).await {
        Ok(picked) => picked,
        Err(refusal) => return refusal,
    };

    let arrival = store::Arrival {
        born,
        repo_id: arriving.repo,
        branch: arriving.branch.clone(),
        branch_named: arriving.branch_named,
        naming: arriving.naming,
        state: crate::ui::state_of(arriving.state),
        rank: arriving.rank.clone(),
        grilling,
        implementation,
        review,
    };

    match store::arrive(&state.pool, &arrival).await {
        Ok(Some(id)) => {
            tracing::info!(
                conversation_id = id,
                born = arriving.born.id,
                on = arriving.born.device,
                "a Conversation transferred from a member landed here",
            );

            // The sidebar of every browser open on this device, and of every
            // member holding this device's list: there is a Conversation here
            // that was not here a moment ago.
            state
                .nudges
                .announce(verkstead_schema::Nudge::Conversations);

            Json(Arrived { id }).into_response()
        }

        Ok(None) => (
            StatusCode::BAD_REQUEST,
            format!(
                "no repository is registered on this device under the id {repo} the transfer \
                 named\n",
                repo = arriving.repo,
            ),
        )
            .into_response(),

        Err(why) => {
            tracing::error!(error = ?why, "a Conversation a member is transferring here could not be written down");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "the Conversation could not be written down on this device\n",
            )
                .into_response()
        }
    }
}

/// `DELETE /api/peer/v1/transfers/{id}` — the device that sent this copy taking
/// it back, its move having failed before it was finished.
///
/// **Whatever state it is in, and without the archive behind it** — see
/// [`store::sweep_arrival`], where that is argued. A copy that never finished
/// arriving was never the human's to look at, and the alternative to sweeping it
/// is a Conversation nobody can account for.
///
/// An id naming nothing answers as well as one naming something: the sender is
/// asking for this not to be here, and it is not here.
pub(crate) async fn sweep(State(state): State<AppState>, Path(id): Path<i64>) -> Response {
    match store::sweep_arrival(&state.pool, id).await {
        Ok(swept) => {
            tracing::info!(
                conversation_id = id,
                outcome = ?swept,
                "a member took back a Conversation whose transfer here failed",
            );

            state
                .nudges
                .announce(verkstead_schema::Nudge::Conversations);

            StatusCode::OK.into_response()
        }

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "a half-transferred Conversation could not be swept");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "the Conversation could not be swept from this device\n",
            )
                .into_response()
        }
    }
}

/// The three roles, each read into an id of this device's own.
///
/// `Err` is the refusal to answer with, naming the Profile that has no row here:
/// the sending device picked the pair off its own rows, and a mirror it holds
/// that this device does not is a cluster that has not finished agreeing with
/// itself.
async fn resolved(
    state: &AppState,
    arriving: &ConversationAcross,
) -> Result<
    (
        store::ArrivingPicked,
        store::ArrivingPicked,
        store::ArrivingPicked,
    ),
    Response,
> {
    // One read of the registry for all three, rather than a lookup apiece: a
    // Conversation names three Pairings and they are very often one account.
    let profiles = match store::profiles(&state.pool).await {
        Ok(profiles) => profiles,
        Err(why) => {
            tracing::error!(error = ?why, "this device's Profiles could not be read for an arriving Conversation");

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device's Agent Profiles could not be read\n",
            )
                .into_response());
        }
    };

    let mut read = [
        store::ArrivingPicked::Nothing,
        store::ArrivingPicked::Nothing,
        store::ArrivingPicked::Nothing,
    ];

    for (at, picked) in [
        &arriving.grilling,
        &arriving.implementation,
        &arriving.review,
    ]
    .into_iter()
    .enumerate()
    {
        read[at] = match picked {
            PickedAcross::Nothing => store::ArrivingPicked::Nothing,
            PickedAcross::Skipped => store::ArrivingPicked::Skipped,

            PickedAcross::Under(pairing) => {
                let Some(profile_id) = here(&profiles, &state.device, &pairing.profile) else {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!(
                            "this device holds no Agent Profile for the one at home on {device} \
                             as {id}\n",
                            device = pairing.profile.device,
                            id = pairing.profile.id,
                        ),
                    )
                        .into_response());
                };

                store::ArrivingPicked::Under {
                    profile_id,
                    model: pairing.model.clone(),
                }
            }
        };
    }

    let [grilling, implementation, review] = read;

    Ok((grilling, implementation, review))
}

/// Which row of this device's `profiles` is the Profile a transfer names, where
/// one is.
///
/// Two ways to be it, and they are the same fact read from the two sides: the
/// account is at home **here**, in which case the pair names this device and one
/// of its own rows; or it is at home on some other machine, in which case what
/// this device holds is a **mirror** of it, marked with that machine and the id
/// it has there — see [`crate::mirroring`].
///
/// One of this device's own rows is never matched by the mirror arm and a mirror
/// is never matched by the first, which is what keeps a Profile from being read
/// as its own mirror on the machine it is at home on.
fn here(profiles: &[store::Profile], device: &str, named: &ProfileAcross) -> Option<i64> {
    profiles
        .iter()
        .find(|profile| match &profile.mirror {
            None => named.device == device && named.id == profile.id,
            Some(mirror) => mirror.device == named.device && mirror.id == named.id,
        })
        .map(|profile| profile.id)
}

/// The routes, over the state the workbench answers out of — mounted by
/// [`super::workbench::served`], where the Repos and the harnesses beside them
/// are mounted and for their reason: what they write is *this* device's record.
pub(crate) fn route() -> Router<AppState> {
    Router::new()
        .route(TRANSFERS, post(take))
        .route(ONE_TRANSFER, delete(sweep))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It is a member's call, so it goes where a member's calls go rather than
    /// through any of the three the gate stands aside for: a stranger does not
    /// write Conversations into somebody's database.
    #[test]
    fn the_transfers_are_not_one_of_the_un_gated_three() {
        for path in [TRANSFERS, ONE_TRANSFER] {
            assert_ne!(path, super::super::IDENTITY);
            assert_ne!(path, super::super::joining::JOIN);
            assert_ne!(path, super::super::exchange::SETTLED);
        }
    }

    /// And outside the three prefixes this device keeps to itself, which a
    /// route served on the one namespace a member never reaches would be
    /// nothing at all.
    #[test]
    fn the_transfers_are_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            for path in [TRANSFERS, ONE_TRANSFER] {
                assert!(
                    !path.starts_with(prefix),
                    "{path} would be held back with {prefix}/",
                );
            }
        }
    }
}
