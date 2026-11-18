//! An installed Plugin's **`secret` settings**, handed to a member for the copy
//! it keeps in its own `secrets.yaml` (ADR-0030, Set 1769, Q43).
//!
//! **The answering half of the copy**, as [`super::mcp_servers`] is of an MCP
//! server's header values, and for its reasons: the record names each secret
//! by a digest, every device holds its own copy, and this answers for any
//! plugin this device holds a live record of with whatever its `secrets.yaml`
//! holds under the uuid — see [`crate::replicating::plugins::copied`].
//!
//! **Gated to members like everything else on this listener**: what it hands
//! over is a secret, and no page draws it.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use verkstead_render::PluginSecretValues;

use crate::AppState;
use crate::replicating::plugins::Plugins;
use crate::store::replicated;

/// Where a member asks after a plugin's secrets, by its uuid. Not under
/// `/api/ui/`, for the token's reason — see [`super::authors::TOKEN`].
pub const SECRETS: &str = "/api/peer/v1/plugins/{plugin}/secrets";

/// `GET /api/peer/v1/plugins/{plugin}/secrets` — every secret `secrets.yaml`
/// holds for a plugin, by key, and none where it holds none. A uuid this
/// device holds no live record of is `404`.
pub(crate) async fn held(State(state): State<AppState>, Path(plugin): Path<String>) -> Response {
    let live = match replicated::live_records::<Plugins>(&state.pool).await {
        Ok(records) => records.iter().any(|record| record.uuid == plugin),
        Err(why) => {
            tracing::error!(
                error = ?why,
                "a member asked after a plugin's secrets and the records could not be read",
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own records\n",
            )
                .into_response();
        }
    };
    if !live {
        return (
            StatusCode::NOT_FOUND,
            "this device holds no plugin under that uuid\n",
        )
            .into_response();
    }

    let settings = state.settings.clone();
    let read = tokio::task::spawn_blocking(move || PluginSecretValues {
        secrets: settings
            .secrets()
            .plugin_secrets_of(&plugin)
            .cloned()
            .unwrap_or_default(),
    })
    .await;

    match read {
        Ok(values) => Json(values).into_response(),
        Err(why) => {
            tracing::error!(
                error = ?why,
                "a member asked after a plugin's secrets and the settings could not be read",
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own settings\n",
            )
                .into_response()
        }
    }
}

/// The route, mounted by [`super::workbench::served`] beside the MCP servers'.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(SECRETS, get(held))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A member's call, gated like the rest: what it hands over is a secret.
    #[test]
    fn the_secrets_are_not_one_of_the_un_gated_three() {
        assert_ne!(SECRETS, super::super::IDENTITY);
        assert_ne!(SECRETS, super::super::joining::JOIN);
        assert_ne!(SECRETS, super::super::exchange::SETTLED);
    }

    /// And outside the prefixes this device keeps to itself.
    #[test]
    fn the_secrets_are_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            assert!(
                !SECRETS.starts_with(prefix),
                "{SECRETS} is held back with {prefix}/"
            );
        }
    }
}
