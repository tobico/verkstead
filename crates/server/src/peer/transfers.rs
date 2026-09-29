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
//! **Which is why nothing here starts anything.** Everything the sending device
//! sends before its own copy is marked is a thing it may take back, and a session
//! launched in a checkout that was about to be swept would be an agent working in
//! a directory nobody could account for. So there is a last call — see
//! [`arrived`] — which the sending device makes once the mark is written, and
//! *that* is the one this device acts on: a Notice saying which machine the work
//! came from, and **Resume** pressed by this device for itself.
//!
//! **Gated to members like everything else on this listener.** Writing a
//! Conversation into somebody's database is not a stranger's press:
//! [`super::router`] puts [`super::members_only`] over this, and a caller whose
//! certificate this device holds no membership for never reaches it.

use std::collections::HashMap;

use axum::Json;
use axum::Router;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, post};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use verkstead_render::{Arrived, CameFrom, ConversationAcross, PickedAcross, ProfileAcross};

use crate::attachments::Attachments;
use crate::transfers::{AttachedFile, MOST_A_RECORD_IS, RecordAcross};
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

/// And where the record itself lands, once that row is there to land beside it.
///
/// **A leg of its own rather than more of the first call.** The ids everything
/// in a record points at are this device's, and the first of them is the
/// Conversation's own — which is what the first call answered with. So the row
/// goes, this device numbers it, and the record follows against that number.
pub const ONE_TRANSFERS_RECORD: &str = "/api/peer/v1/transfers/{id}/record";

/// And where the device that sent it says the move is over, which is the one call
/// this device may act on — see [`arrived`].
pub const ONE_TRANSFERS_ARRIVAL: &str = "/api/peer/v1/transfers/{id}/arrival";

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

/// `POST /api/peer/v1/transfers/{id}/record` — the Timeline and everything
/// hanging off it, written down against the Conversation this device numbered a
/// moment ago.
///
/// **Every id is this device's by the time it is written.** The rows arrive
/// carrying the numbers they had on the machine they came off — every Verkstead
/// has a Conversation 1 — and the store renumbers them as they land, so an Event
/// referenced by a Capture, a Transcript, a session Pairing or a Question Set
/// comes out pointing at the Event it actually landed as. What the store cannot
/// work out for itself is the two ids that are not this Conversation's: the
/// Repos, which the sending device matched against this registry before it sent
/// anything, and the Agent Profiles, which are named across a cluster by the
/// device each is at home on — see [`resolved`], which is the same reading one
/// call along.
///
/// **The files first, then the rows.** An attachment is a row and bytes both,
/// and the row is what names the file: written the other way round there would
/// be a moment in which a session on this device could be told about a path that
/// is not there yet. A landing that then fails takes the directory back with it,
/// the copy being one the sending device is about to sweep.
pub(crate) async fn written(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(record): Json<RecordAcross>,
) -> Response {
    let profiles = match accounts(&state, &record).await {
        Ok(profiles) => profiles,
        Err(refusal) => return refusal,
    };

    let renaming = store::Renaming {
        repos: record.repos.iter().copied().collect(),
        profiles,
    };

    if let Err(why) = kept(&state, id, &record.files).await {
        tracing::error!(error = ?why, conversation_id = id, "the files on a Conversation a member is transferring here could not be written");

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "the attached files could not be written down on this device\n",
        )
            .into_response();
    }

    match store::land(&state.pool, id, &record.slice, &renaming).await {
        Ok(()) => {
            tracing::info!(
                conversation_id = id,
                tables = record.slice.tables.len(),
                files = record.files.len(),
                "the record of a Conversation transferred from a member landed here",
            );

            // The Timeline on every browser open on this device, and the sidebar
            // of every member holding its list: there is work here now rather
            // than the empty row the first leg left.
            state
                .nudges
                .announce(verkstead_schema::Nudge::Conversation { conversation: id });
            state
                .nudges
                .announce(verkstead_schema::Nudge::Conversations);

            StatusCode::OK.into_response()
        }

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "the record of a Conversation a member is transferring here could not be written down");

            Attachments::under(&state.data_dir).remove(id);

            (StatusCode::BAD_REQUEST, format!("{why:#}\n")).into_response()
        }
    }
}

/// `POST /api/peer/v1/transfers/{id}/arrival` — the device that sent the work
/// saying the move is over, and this device taking it up.
///
/// **The one leg after the commit point, and the only one this device may act
/// on.** Every call before it is in front of the mark the sending device writes:
/// a move that falls over there is swept — see [`sweep`] — and a session started
/// in a checkout that was about to be taken back would be an agent working in a
/// directory nobody could account for. By the time this arrives the sending
/// device has let go, so the work is *here*, and two things follow from that.
///
/// **A Notice saying where it came from**, so the record reads as one story
/// across the two machines rather than as a Conversation that appeared from
/// nowhere. By the name this device knows that machine by, the way every sentence
/// about a member is written — see [`crate::relaying::called`].
///
/// **And Resume, pressed by this device for itself.** Resume is the one standing
/// way in: it asks what *ought* to be running now, from the lifecycle the
/// Conversation is in and what the branch has written, which is exactly the
/// question a Conversation that has just arrived poses. See
/// [`crate::resume::on_arrival`], where the refusals are written down.
///
/// **`Ok` whatever the Resume decided**, because there is nothing the sending
/// device could do with the answer: its copy is already the tombstone and the
/// work is already here. What a refusal leaves is a stop on this device's own
/// Timeline, which is where the human who has to act on it is looking.
pub(crate) async fn arrived(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(from): Json<CameFrom>,
) -> Response {
    if !matches!(store::load_conversation(&state.pool, id).await, Ok(Some(_))) {
        return (
            StatusCode::BAD_REQUEST,
            "no Conversation on this device has that id\n",
        )
            .into_response();
    }

    let named = crate::relaying::called(state.devices.as_ref(), &from.device).await;

    tracing::info!(
        conversation_id = id,
        device = from.device,
        "a member says the Conversation it moved here has finished arriving",
    );

    let noted = store::note(
        &state.pool,
        id,
        &format!(
            "This Conversation was moved onto this device from **{named}**. Its branch, its \
             Worktree and whatever was uncommitted in it are here, and the record above it \
             came across with them.",
        ),
    )
    .await;

    if let Err(error) = noted {
        tracing::error!(error = ?error, conversation_id = id, "saying on the Timeline where a Conversation arrived from failed");
    }

    state
        .nudges
        .announce(verkstead_schema::Nudge::Conversation { conversation: id });

    crate::resume::on_arrival(&state, id, &named).await;

    StatusCode::OK.into_response()
}

/// Which of this device's Agent Profiles each one the record names is, where
/// this device has one at all.
///
/// [`resolved`]'s reading over a whole record rather than over three Pairings,
/// and with the missing ones left out rather than refused: what names a Profile
/// here is a Steer somebody ran a year ago, and an account deleted since is a
/// name no machine can keep. The column lands empty and the rest of the Steer
/// stands — see `store::slices`.
async fn accounts(state: &AppState, record: &RecordAcross) -> Result<HashMap<i64, i64>, Response> {
    if record.profiles.is_empty() {
        return Ok(HashMap::new());
    }

    let profiles = match store::profiles(&state.pool).await {
        Ok(profiles) => profiles,
        Err(why) => {
            tracing::error!(error = ?why, "this device's Profiles could not be read for an arriving record");

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device's Agent Profiles could not be read\n",
            )
                .into_response());
        }
    };

    Ok(record
        .profiles
        .iter()
        .filter_map(|(there, named)| Some((*there, here(&profiles, &state.device, named)?)))
        .collect())
}

/// The attached files, written into this device's own directory for this
/// Conversation.
///
/// Under the id this device gave it, which is what makes the path one its own
/// sessions are given — see [`crate::attachments`]. Each keeps the name the row
/// says it was stored under: the directory is one nothing has written to yet, so
/// nothing is counted up and the row and the file agree.
///
/// Blocking, off the runtime's threads: an attachment is as much as thirty-two
/// megabytes.
async fn kept(state: &AppState, id: i64, files: &[AttachedFile]) -> anyhow::Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    let directories = Attachments::under(&state.data_dir);
    let files: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|file| {
            Ok((
                file.name.clone(),
                STANDARD.decode(&file.bytes).map_err(|why| {
                    anyhow::anyhow!("the bytes of {name} did not read: {why}", name = file.name)
                })?,
            ))
        })
        .collect::<anyhow::Result<_>>()?;

    tokio::task::spawn_blocking(move || {
        for (name, bytes) in files {
            directories.keep(id, &name, &bytes)?;
        }

        anyhow::Ok(())
    })
    .await?
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
    // Where its checkout went, read before the rows that name it are taken away:
    // a directory nothing points at is one nobody would ever find again. Nothing
    // is there for a copy swept before the git leg ran, which is most of them.
    let checkout = match store::closable(&state.pool, id).await {
        Ok(checkout) => checkout,

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "reading what a half-transferred Conversation had checked out failed");

            None
        }
    };

    match store::sweep_arrival(&state.pool, id).await {
        Ok(swept) => {
            tracing::info!(
                conversation_id = id,
                outcome = ?swept,
                "a member took back a Conversation whose transfer here failed",
            );

            // And the files that came with it, which are bytes rather than rows
            // and so are no part of the walk above. A copy that never finished
            // arriving was never the human's to look at, and a directory of
            // somebody else's attachments under an id nothing names is exactly
            // the leftover this sweep is for.
            Attachments::under(&state.data_dir).remove(id);

            // And the checkouts, for that reason again: a Worktree is a directory
            // this device made for work that turned out never to have arrived.
            // The Companions' among them, each being as much this device's making
            // as the Conversation's own. The branches stay — they are the work
            // itself, and a bundle that landed is history this repository now has
            // whatever became of the move.
            if let Some(checkout) = checkout {
                let mut removing: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();

                if let Some(worktree) = checkout.worktree {
                    removing.push((checkout.repo, worktree));
                }

                for companion in checkout.companions {
                    if let Some(worktree) = companion.worktree {
                        removing.push((companion.repo, worktree));
                    }
                }

                let _ = tokio::task::spawn_blocking(move || {
                    for (repo, worktree) in removing {
                        crate::worktrees::remove(&repo, &worktree);
                    }
                })
                .await;
            }

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
        .route(
            ONE_TRANSFERS_RECORD,
            // Over the router's own default, the way the Question Set route and
            // the memory store's raise theirs: a record is a Conversation's
            // whole Timeline, and the bound over it is the record's own.
            post(written).layer(DefaultBodyLimit::max(MOST_A_RECORD_IS)),
        )
        .route(ONE_TRANSFERS_ARRIVAL, post(arrived))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It is a member's call, so it goes where a member's calls go rather than
    /// through any of the three the gate stands aside for: a stranger does not
    /// write Conversations into somebody's database.
    #[test]
    fn the_transfers_are_not_one_of_the_un_gated_three() {
        for path in [
            TRANSFERS,
            ONE_TRANSFER,
            ONE_TRANSFERS_RECORD,
            ONE_TRANSFERS_ARRIVAL,
        ] {
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
            for path in [
                TRANSFERS,
                ONE_TRANSFER,
                ONE_TRANSFERS_RECORD,
                ONE_TRANSFERS_ARRIVAL,
            ] {
                assert!(
                    !path.starts_with(prefix),
                    "{path} would be held back with {prefix}/",
                );
            }
        }
    }
}
