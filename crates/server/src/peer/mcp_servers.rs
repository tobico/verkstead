//! A declared MCP server's **header values**, handed to a member for the copy
//! it keeps in its own `secrets.yaml` (ADR-0021, ADR-0028).
//!
//! **The answering half of the values' copy**, as [`super::authors`] is of an
//! Author's token. Every device holds every declaration as a replicated
//! record, and a session attached to one is handed this device's copy of its
//! values — held here rather than fetched at launch, so that a device that is
//! off does not stop a session over here. The record says which values are
//! held, by a digest of each; this is what the copy is fetched from — see
//! [`crate::replicating::mcp_servers::copied`].
//!
//! **From whoever holds them.** There is no home: this answers for any
//! declaration this device holds a live record of, with whatever its
//! `secrets.yaml` holds under the uuid — values set here and a copy fetched
//! from somewhere else alike — so they reach every device while the one they
//! were set on is off.
//!
//! **Gated to members like everything else on this listener**, the token's
//! way and for its reason: what it hands over is a credential, and no page
//! draws it.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use verkstead_render::McpHeaderValues;

use crate::AppState;

/// Where a member asks after the header values of a declaration, by its
/// uuid — which is its uuid on every device. Not under `/api/ui/`, for the
/// token's reason — see [`super::authors::TOKEN`].
pub const HEADERS: &str = "/api/peer/v1/mcp-servers/{server}/headers";

/// `GET /api/peer/v1/mcp-servers/{server}/headers` — every header value
/// `secrets.yaml` holds for a declaration, by header name, and none where it
/// holds none.
///
/// **Read at the moment it is asked for**, off the two files: a rotation saved
/// here is what the next call answers.
///
/// **A uuid this device holds no live record of is `404`** — one nobody here
/// has heard of, or one removed, whose values went with it.
pub(crate) async fn held(State(state): State<AppState>, Path(server): Path<String>) -> Response {
    let settings = state.settings.clone();

    let read = tokio::task::spawn_blocking(move || {
        // The table's live declarations, which are the ones the settings hold.
        settings
            .config()
            .mcp_servers()
            .iter()
            .any(|declared| declared.id() == Some(server.as_str()))
            .then(|| McpHeaderValues {
                headers: settings
                    .secrets()
                    .mcp_headers_of(&server)
                    .cloned()
                    .unwrap_or_default(),
            })
    })
    .await;

    match read {
        Ok(Some(values)) => Json(values).into_response(),

        Ok(None) => (
            StatusCode::NOT_FOUND,
            "this device holds no MCP server under that uuid\n",
        )
            .into_response(),

        Err(why) => {
            tracing::error!(
                error = ?why,
                "a member asked after an MCP server's header values and the settings could \
                 not be read",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own settings\n",
            )
                .into_response()
        }
    }
}

/// The route, over the state the workbench answers out of — mounted by
/// [`super::workbench::served`] beside the token's.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(HEADERS, get(held))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A member's call, so it goes where a member's calls go rather than
    /// through any of the three the gate stands aside for: what it hands over
    /// is a credential.
    #[test]
    fn the_values_are_not_one_of_the_un_gated_three() {
        assert_ne!(HEADERS, super::super::IDENTITY);
        assert_ne!(HEADERS, super::super::joining::JOIN);
        assert_ne!(HEADERS, super::super::exchange::SETTLED);
    }

    /// And outside the prefixes this device keeps to itself, or nothing could
    /// ever reach it.
    #[test]
    fn the_values_are_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            assert!(
                !HEADERS.starts_with(prefix),
                "{HEADERS} is held back with {prefix}/"
            );
        }
    }
}
