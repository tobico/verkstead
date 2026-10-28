//! An Author's **token**, handed to a member for the copy it keeps in its own
//! `secrets.yaml` (ADR-0025, ADR-0028).
//!
//! **The answering half of the token copy.** Every device holds every Author as
//! a replicated record, and a Repo pointed at one commits and pushes from this
//! device's copy of its token — held here rather than fetched at the moment of
//! need, so that a device that is off does not stop a Repo over here. The
//! record says whether a token is held and which; this is what the copy is
//! fetched from — see [`crate::replicating::authors::copied`].
//!
//! **From whoever holds one.** There is no home: this answers for any Author
//! this device holds a live record of, with whatever its `secrets.yaml` holds
//! under the uuid — a token set here and a copy fetched from somewhere else
//! alike — so a token reaches every device while the one it was set on is off.
//!
//! **A member's own call rather than the viewer's**, beside
//! [`super::account`], whose shape it is: what it hands over is a credential,
//! and no page draws it.
//!
//! **Gated to members like everything else on this listener**: [`super::router`]
//! puts [`super::members_only`] over it, so it travels over the mutually
//! authenticated link the account mirror's login already travels, and a caller
//! whose certificate this device holds no membership for never reaches it.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use verkstead_render::AuthorToken;

use crate::AppState;

/// Where a member asks after the token of an Author, by its uuid — which is its
/// uuid on every device.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace and is drawn by a
/// browser. The account's shape, for its reason.
pub const TOKEN: &str = "/api/peer/v1/authors/{author}/token";

/// `GET /api/peer/v1/authors/{author}/token` — the token `secrets.yaml` holds
/// for an Author, or `null` where it holds none.
///
/// **Read at the moment it is asked for**, off the two files, as everything
/// about an Author is: a rotation saved here is what the next call answers.
///
/// **A uuid this device holds no live record of is `404`** — one nobody here
/// has heard of, or one removed, whose copy went with it.
pub(crate) async fn held(State(state): State<AppState>, Path(author): Path<String>) -> Response {
    let settings = state.settings.clone();

    let read = tokio::task::spawn_blocking(move || {
        // The table's live Authors, which are the ones the settings hold.
        settings
            .config()
            .author(&author)
            .is_some()
            .then(|| AuthorToken {
                token: settings.secrets().author_token(&author).map(str::to_owned),
            })
    })
    .await;

    match read {
        Ok(Some(token)) => Json(token).into_response(),

        Ok(None) => (
            StatusCode::NOT_FOUND,
            "this device holds no Author under that uuid\n",
        )
            .into_response(),

        Err(why) => {
            tracing::error!(
                error = ?why,
                "a member asked after an Author's token and the settings could not be read",
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
/// [`super::workbench::served`] for the reason the account beside it is.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(TOKEN, get(held))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A member's call, so it goes where a member's calls go rather than
    /// through any of the three the gate stands aside for: what it hands over
    /// is a credential.
    #[test]
    fn the_token_is_not_one_of_the_un_gated_three() {
        assert_ne!(TOKEN, super::super::IDENTITY);
        assert_ne!(TOKEN, super::super::joining::JOIN);
        assert_ne!(TOKEN, super::super::exchange::SETTLED);
    }

    /// And outside the prefixes this device keeps to itself, or nothing could
    /// ever reach it.
    #[test]
    fn the_token_is_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            assert!(
                !TOKEN.starts_with(prefix),
                "{TOKEN} is held back with {prefix}/"
            );
        }
    }
}
