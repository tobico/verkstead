//! A stage's join, as its home answers it: a member holding the live copy of a
//! stage born here, at that stage's finish, asking whether it may join its
//! roadmap's chain and what it joins on top of (ADR-0029, *Home drives a roadmap
//! across devices*).
//!
//! **The chain and the queue are home's**, read off the record the tombstone is
//! a row of — so the far device's hold in front of its finish asks here rather
//! than reading its own copy of either. See [`crate::joins`], whose
//! [`crate::joins::asked`] is the answer.
//!
//! **Only the device the work is on may ask**, for the report's reason, and
//! under the same check — see [`super::reports::tombstone_of`]. A member's own
//! call, so it stands inside [`super::gate`] beside the reports. And an ask to
//! an **Heir** adopts the stage as a report would, from what it says the stage
//! is: the ask is what the stage is waiting on, and a report that has not
//! landed is no reason for it to wait longer.

use axum::Json;
use axum::Router;
use axum::extract::{ConnectInfo, State};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use verkstead_render::StageJoining;

use crate::AppState;

use super::Caller;

/// Where a stage away from home asks whether it may join.
///
/// Naming no device and no Conversation, for the reports' reason: the caller is
/// read off the handshake, and the stage is named by its Birth Key in the body.
pub const CHAINS: &str = "/api/peer/v1/chains";

/// `POST /api/peer/v1/chains` — queue the stage's tombstone to join, and answer
/// what is ahead of it and the chain it would join.
pub(crate) async fn asked(
    State(state): State<AppState>,
    ConnectInfo(caller): ConnectInfo<Caller>,
    Json(asking): Json<StageJoining>,
) -> Response {
    // An ask carries its state inside what it says the stage is, having none of
    // its own; one that says no state adopts nothing, and is answered as an
    // ask about a stage held here.
    let adopting = asking.adopting.as_ref().and_then(|adopting| {
        adopting
            .state
            .map(|state| (adopting, crate::ui::state_of(state)))
    });

    let tombstone =
        match super::reports::tombstone_of(&state, &caller, &asking.born, adopting).await {
            Ok(tombstone) => tombstone,
            Err(refusal) => return refusal,
        };

    let answer = crate::joins::asked(&state, tombstone).await;

    tracing::info!(
        tombstone,
        ahead = answer.ahead.len(),
        "a stage away from home asked whether it may join its chain",
    );

    Json(answer).into_response()
}

/// The route, over the state the workbench answers out of — mounted by
/// [`super::workbench::served`] beside the reports.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(CHAINS, post(asked))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A member's call, so not one of the three the gate stands aside for.
    #[test]
    fn the_chains_are_not_one_of_the_un_gated_three() {
        assert_ne!(CHAINS, super::super::IDENTITY);
        assert_ne!(CHAINS, super::super::joining::JOIN);
        assert_ne!(CHAINS, super::super::exchange::SETTLED);
    }

    /// And outside the prefixes this device keeps to itself.
    #[test]
    fn the_chains_are_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            assert!(
                !CHAINS.starts_with(prefix),
                "the chains would be held back with {prefix}/",
            );
        }
    }
}
