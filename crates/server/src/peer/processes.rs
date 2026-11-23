//! Which Processes this device could run a Conversation on, as a member is told
//! of them (ADR-0020, *Transfer*; ADR-0030, *Process*).
//!
//! **The answering half of a transfer's preflight, for the Process.** Every
//! Process is data in a plugin, so a Conversation moved onto a device that has
//! no plugin providing its Process is one nothing over there could drive — it
//! would land and stop with the plugin's trouble as its Notice. Which plugins
//! loaded is each device's own reading of its own copy (see
//! `crate::replicating::plugins`), so only the far end can answer, and
//! [`crate::preflight`] is the end that asks and composes the sentence.
//!
//! **The whole list rather than the one asked after**, which is the harnesses'
//! shape beside it — see [`super::harnesses`]: a handful of rows, the caller
//! picks out the one its Conversation is on, and a reading with nothing to send
//! up is a `GET`.
//!
//! **Gated to members like everything else on this listener**, for the reason
//! the harnesses are: what somebody has installed is not a stranger's business.

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use verkstead_render::ProvidedProcess;

use crate::AppState;

/// Where a member asks after the Processes this device holds.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace: no browser makes
/// this call. It names nobody, there being nothing to fill in — a device asks
/// for the lot and picks out what it wanted, the harnesses' shape beside it.
pub const PROCESSES: &str = "/api/peer/v1/processes";

/// The most a device's Processes may be before the answer is dropped: **sixty-
/// four kilobytes**.
///
/// Two short strings a row, so this is room for hundreds of them. What it is
/// for is the case every bound across a link is for: a machine on the far end
/// that answers and then writes without stopping — see
/// [`crate::preflight::MOST_THE_HARNESSES_ARE`], which is the same bound one
/// reading along.
pub const MOST_THE_PROCESSES_ARE: usize = 64 * 1024;

/// `GET /api/peer/v1/processes` — every Process a plugin on this device
/// provides, by the plugin's uuid and the id its manifest gives it.
///
/// **Held rather than offered**, which is the question a transfer is asking: a
/// plugin switched off still drives the Conversations already on its Process
/// (Set 1764, Q20), so work may be moved onto a device whose tick is off and
/// carries on there. What is refused is a device with no such plugin at all.
pub(crate) async fn here(State(state): State<AppState>) -> Response {
    let held: Vec<ProvidedProcess> = crate::processes::listed(&state)
        .await
        .into_iter()
        .map(|loaded| crate::ui::provided(&loaded.whose()))
        .collect();

    Json(held).into_response()
}

/// The route, over the state the workbench answers out of — mounted where the
/// harnesses beside it are, and for their reason: it answers out of what this
/// device has loaded.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(PROCESSES, get(here))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It is a member's call, so it goes where a member's calls go rather than
    /// through any of the three the gate stands aside for.
    #[test]
    fn the_processes_are_not_one_of_the_un_gated_three() {
        assert_ne!(PROCESSES, super::super::IDENTITY);
        assert_ne!(PROCESSES, super::super::joining::JOIN);
        assert_ne!(PROCESSES, super::super::exchange::SETTLED);
    }
}
