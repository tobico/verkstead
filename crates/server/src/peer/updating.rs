//! A member told to update itself, and asked how its Update stands (ADR-0027,
//! *A device updates itself*).
//!
//! **The pressing device tells and the member does the whole of it.** A press
//! of Update on a member's row arrives here, and what this device does with it
//! is exactly what a press on its own row does: refuse by name where its
//! install can't update itself, or mark itself updating and say
//! [`crate::updating::ASKING`] to its own desktop app, which fetches its own
//! latest stable release. No bits cross the link — pushing them was the road
//! not taken.
//!
//! **A member's own route rather than the viewer's relayed.** Stage 04's press
//! is under `/api/ui/devices/`, which is one of the prefixes never served to a
//! member — see [`super::workbench::KEPT_TO_ITSELF`] — and the progress and
//! failure the desktop app posts stay there, local. So the press and the
//! reading are a pair of their own here, inside the Member Gate with the news
//! and the renewal, and the path names nobody: the device being told is the
//! device answering.
//!
//! **The refusal is a sentence naming this device**, the way the Annex's
//! `running_there` names its box: the pressing device draws it on this
//! device's row as it came. Plain text, as the rest of this listener refuses.

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use verkstead_render::{MemberUpdate, UpdateTold};

use crate::AppState;
use crate::device::Devices;

/// Where a member is told to update itself (`POST`), and asked how its Update
/// stands (`GET`).
pub const UPDATE: &str = "/api/peer/v1/update";

/// `POST /api/peer/v1/update` — *update yourself*: this device's own row's
/// press, made by a member.
///
/// Answered with this device's Update as it now stands — *updating…* from here
/// — or `409` and the sentence saying why this install is updated by hand. A
/// press while one is going reads the one there is, as a second press on its
/// own row does.
pub(crate) async fn told(State(state): State<AppState>, Json(told): Json<UpdateTold>) -> Response {
    let Some(devices) = state.devices.as_ref() else {
        return no_device();
    };

    if let Some(why) = crate::updating::refusal(devices.install(), &devices.name()) {
        tracing::info!(
            install = ?devices.install(),
            "a member told this device to update itself, and its install is updated by hand",
        );

        return (StatusCode::CONFLICT, format!("{why}\n")).into_response();
    }

    tracing::info!(
        release = told.release.as_deref().unwrap_or("unknown"),
        "a member told this device to update itself",
    );

    if devices.updating().begin() {
        state.nudges.announce(verkstead_schema::Nudge::Devices);
    }

    Json(standing(devices)).into_response()
}

/// `GET /api/peer/v1/update` — this device's Update as it stands, which the
/// member that pressed draws on this device's row.
pub(crate) async fn asked(State(state): State<AppState>) -> Response {
    let Some(devices) = state.devices.as_ref() else {
        return no_device();
    };

    Json(standing(devices)).into_response()
}

/// The two readings this device's own row is drawn with.
fn standing(devices: &Devices) -> MemberUpdate {
    MemberUpdate {
        updating: devices.updating().reading(),
        trouble: devices.updating().trouble(),
    }
}

/// A router stood up without a Data Directory, which invented no identity and
/// has no install to say anything about — every router but a served one.
fn no_device() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "this device cannot say what it was installed as\n",
    )
        .into_response()
}

/// The route, over the state the workbench answers out of — mounted by
/// [`super::workbench::served`] beside the news, for that module's reason: the
/// Update is this state's.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(UPDATE, get(asked).post(told))
}
