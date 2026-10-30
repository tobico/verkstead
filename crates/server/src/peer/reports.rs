//! A stage's report, as its home answers it: a member holding the live copy of
//! a stage born here, saying where that stage now stands (ADR-0029, *Home
//! drives a roadmap across devices*).
//!
//! **Home keeps the roadmap.** Every stage is made on the device driving its
//! roadmap, and a stage moved away leaves a tombstone here that the standings,
//! the places and the chain go on reading. So what arrives is written onto that
//! tombstone — see [`store::follow`] — and the roadmap is read where it always
//! was. Where the report moved the tombstone into Done, the roadmap is carried
//! on from it exactly as a settle here would, and what that would have said on
//! the tombstone's Timeline is the answer: nobody opens a tombstone, so the copy
//! says it on its own — see [`crate::continuing::reported`].
//!
//! **Only the device the work is on may report.** The tombstone names it, and
//! the certificate the call was made under is what says which device is
//! calling — see [`super::Members::presenting`]. A report about work this
//! device holds no tombstone for, or one whose tombstone names some other
//! machine, is refused: a device that has handed the work on has nothing left
//! to say about it.
//!
//! **A member's own call, so it stands inside [`super::gate`]**, beside the
//! news, which is its nearest precedent: a member saying something about its
//! own work.

use axum::Json;
use axum::Router;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use verkstead_render::{StageReport, StageReported};
use verkstead_schema::Nudge;

use crate::{AppState, store};

use super::Caller;

/// Where the device holding a stage away from home says where it stands.
///
/// Naming no device and no Conversation, for the news's reason: the caller is
/// read off the handshake, and the stage is named by its Birth Key in the body,
/// which is the one name both ends share for it.
pub const REPORTS: &str = "/api/peer/v1/reports";

/// `POST /api/peer/v1/reports` — write a stage's standing onto its tombstone,
/// and answer with what carrying the roadmap on said.
pub(crate) async fn heard(
    State(state): State<AppState>,
    ConnectInfo(caller): ConnectInfo<Caller>,
    Json(report): Json<StageReport>,
) -> Response {
    let Some(presented) = caller.fingerprint() else {
        return refused(
            StatusCode::FORBIDDEN,
            "a device that presented no certificate has no stage to report on",
        );
    };

    let Some(devices) = state.devices.as_ref() else {
        tracing::error!("a stage's report arrived at a device that does not know what it is");

        return refused(
            StatusCode::INTERNAL_SERVER_ERROR,
            "this device cannot say which of its members is calling",
        );
    };

    let member = match devices.membership().presenting(&presented).await {
        Ok(Some(member)) => member,

        // Through the gate and unlinked a moment later: nobody this device
        // would take a report from.
        Ok(None) => {
            return refused(
                StatusCode::FORBIDDEN,
                "this device holds no membership for the caller",
            );
        }

        Err(why) => {
            tracing::error!(%why, "the membership a stage's report is attributed to could not be read");

            return refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read the membership",
            );
        }
    };

    if report.born.device != state.device {
        return refused(
            StatusCode::NOT_FOUND,
            "that stage was not born on this device, so this is not its home",
        );
    }

    let born = store::Birth {
        device: report.born.device.clone(),
        id: report.born.id,
    };

    let tombstone = match store::born_as(&state.pool, &born).await {
        Ok(Some(id)) => id,

        Ok(None) => {
            return refused(
                StatusCode::NOT_FOUND,
                "this device holds no copy of that stage",
            );
        }

        Err(why) => {
            tracing::error!(error = ?why, "looking for the stage a report is about failed");

            return refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own record",
            );
        }
    };

    match store::transferred(&state.pool, tombstone).await {
        Ok(Some(mark)) if mark.device == member.device => {}

        Ok(_) => {
            tracing::warn!(
                tombstone,
                caller = %member.device,
                "a member reported on a stage whose live record is not on it, and was refused",
            );

            return refused(
                StatusCode::CONFLICT,
                "the work on that stage is not on the device reporting it",
            );
        }

        Err(why) => {
            tracing::error!(error = ?why, tombstone, "reading where a stage's live record is failed");

            return refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own record",
            );
        }
    }

    let standing = store::LiveStanding {
        state: crate::ui::state_of(report.state),
        done: report.done,
        wrapped: report.wrapped,
        stopped: report.stopped,
    };

    let followed = match store::follow(&state.pool, tombstone, &standing).await {
        Ok(followed) => followed,

        Err(why) => {
            tracing::error!(error = ?why, tombstone, "writing a stage's standing onto its tombstone failed");

            return refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not write the stage's standing down",
            );
        }
    };

    tracing::info!(
        tombstone,
        device = %member.device,
        state = ?standing.state,
        settled = followed.settled,
        "a stage away from home reported where it stands, and its tombstone follows it",
    );

    // What reads the roadmap here has moved: the card, and the list.
    state.nudges.announce(Nudge::Conversation {
        conversation: tombstone,
    });

    let said = match followed.settled {
        true => crate::continuing::reported(state.clone(), tombstone).await,
        false => Vec::new(),
    };

    Json(StageReported { said }).into_response()
}

/// The route, over the state the workbench answers out of — mounted by
/// [`super::workbench::served`], for the news's reason: the roadmap this writes
/// to is that state's.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(REPORTS, post(heard))
}

/// A refusal on this listener, in the plain text the rest of it refuses in.
fn refused(status: StatusCode, why: &'static str) -> Response {
    (status, format!("{why}\n")).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A member's call, so not one of the three the gate stands aside for.
    #[test]
    fn the_reports_are_not_one_of_the_un_gated_three() {
        assert_ne!(REPORTS, super::super::IDENTITY);
        assert_ne!(REPORTS, super::super::joining::JOIN);
        assert_ne!(REPORTS, super::super::exchange::SETTLED);
    }

    /// And outside the prefixes this device keeps to itself, or nothing could
    /// ever reach it.
    #[test]
    fn the_reports_are_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            assert!(
                !REPORTS.starts_with(prefix),
                "the reports would be held back with {prefix}/",
            );
        }
    }
}
