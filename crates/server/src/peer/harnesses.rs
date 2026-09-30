//! Which harnesses are on this device's `PATH`, as a member is told of them
//! (ADR-0020, *Transfer*).
//!
//! **The answering half of a transfer's preflight.** Before work moves onto a
//! device, the device holding it has to know that every account the
//! Conversation's Pairings name could be launched over there at all — and that
//! is a fact about the far end's filesystem, which only the far end can look at.
//! So this is what it answers out of, and [`crate::preflight`] is the end that
//! asks and composes the sentence.
//!
//! **The whole list rather than the ones asked after.** Four rows is every
//! harness there is, the caller picks out the ones its Pairings want, and a
//! reading with nothing to send up is a `GET` like the Repos beside it — see
//! [`super::repos`], which answers a registry for the same reason.
//!
//! **And it is the onboarding probe's own finding**, under the name a session of
//! that type is launched as: the dependencies step already says whether a
//! harness is on a machine, and a second probe would be a second answer to one
//! question — see [`crate::onboarding::Machine::harnesses`].
//!
//! **Gated to members like everything else on this listener.** What is installed
//! on somebody's machine is not a stranger's business: [`super::router`] puts
//! [`super::members_only`] over this, and a caller whose certificate this device
//! holds no membership for never reaches it.

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use verkstead_render::HarnessThere;

use crate::AppState;

/// Where a member asks after this device's harnesses.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace and is drawn by a
/// browser; and it names nobody, there being nothing to fill in — a device asks
/// for the lot and picks out what it wanted. The same shape the Repos beside it
/// take.
pub const HARNESSES: &str = "/api/peer/v1/harnesses";

/// `GET /api/peer/v1/harnesses` — every harness, and whether a session of that
/// type could be launched on this machine.
///
/// The `PATH` walks go in one blocking task rather than one apiece: they are
/// four lookups against a directory list, and a preflight is asked for once
/// before a move rather than on every page.
pub(crate) async fn here(State(state): State<AppState>) -> Response {
    let machine = state.onboarding.machine().clone();

    let answered = tokio::task::spawn_blocking(move || {
        machine
            .harnesses()
            .into_iter()
            .map(|(agent_type, there)| HarnessThere {
                agent_type: crate::profiles::agent_type(agent_type),
                there,
            })
            .collect::<Vec<_>>()
    })
    .await;

    match answered {
        Ok(answered) => Json(answered).into_response(),

        Err(why) => {
            tracing::error!(
                error = ?why,
                "the harnesses a member asked after could not be looked for",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own harnesses\n",
            )
                .into_response()
        }
    }
}

/// The route, over the state the workbench answers out of.
///
/// **Built here and mounted by [`super::workbench::served`]**, which is where
/// the Repos beside it are mounted and for the same reason: it answers out of
/// the machine this device is on, and a router over a second state would be
/// answering for a second device's `PATH`.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(HARNESSES, get(here))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It is a member's call, so it goes where a member's calls go rather than
    /// through any of the three the gate stands aside for: what is installed on
    /// somebody's machine is not a stranger's business.
    #[test]
    fn the_harnesses_are_not_one_of_the_un_gated_three() {
        assert_ne!(HARNESSES, super::super::IDENTITY);
        assert_ne!(HARNESSES, super::super::joining::JOIN);
        assert_ne!(HARNESSES, super::super::exchange::SETTLED);
    }

    /// And it names no device: a device asks for the whole list and picks out
    /// what its own Pairings want, so there is nothing for a caller to fill in.
    #[test]
    fn the_harnesses_name_no_device() {
        assert!(
            !HARNESSES.contains('{'),
            "the harnesses path carries no segment somebody fills in: {HARNESSES}",
        );
    }

    /// And it is outside the three prefixes this device keeps to itself, which
    /// is what it has to be: a reading served on the one namespace a member is
    /// never served would be a route nothing could ever reach.
    #[test]
    fn the_harnesses_are_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            assert!(
                !HARNESSES.starts_with(prefix),
                "the harnesses would be held back with {prefix}/",
            );
        }
    }
}
