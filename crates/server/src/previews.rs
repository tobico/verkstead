//! Suggesting a Preview: a building session saying how to try its work by
//! hand, by running `verkstead preview <title>` with the command on stdin
//! (ADR-0024).
//!
//! **Only a building session may**, by the Memo's rule and off the same
//! register — see [`crate::memos`]. A grilling has nothing to try and an
//! investigation commits nothing, so every other kind is refused by name,
//! saying which may.
//!
//! **Keyed on its title.** The same title again replaces the command and the
//! description in place — one suggested without a description clears the one
//! that stood, what the agent last said being what stands — and stamps it
//! with the session suggesting it now; a retire takes the title back, and one
//! that does not stand is refused by name. Neither puts anything
//! on the Timeline: both announce the Conversation's Nudge, which is what
//! re-reads the view the Previews are served on.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use verkstead_schema::{ApiError, Nudge, PreviewLeft, PreviewSuggestion};

use crate::AppState;
use crate::memos::{Building, MAY};
use crate::reply::yaml;
use crate::store;

/// `POST /conversations/{conversation}/api/v1/previews` — the session running
/// in this Conversation suggests the [`PreviewSuggestion`] in the body.
///
/// 200 with the title it stands under. 409 where the suggestion is malformed
/// or refused by [`PreviewSuggestion::refused`], where no session is running
/// here, or where the one running is not a building one.
pub(crate) async fn suggest(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
    body: String,
) -> Response {
    let suggestion = match serde_saphyr::from_str::<PreviewSuggestion>(&body) {
        Ok(asked) => {
            PreviewSuggestion::new(&asked.title, &asked.command, asked.description.as_deref())
        }
        Err(error) => {
            return refused(
                conversation_id,
                format!("the preview is not a title and a command in YAML: {error}"),
            );
        }
    };

    if let Some(why) = suggestion.refused() {
        return refused(conversation_id, why);
    }

    let building = match building(&state, conversation_id, "suggest").await {
        Ok(building) => building,
        Err(refusal) => return refusal,
    };

    let left_by = building.left_by();

    match store::suggest_preview(
        &state.pool,
        conversation_id,
        &suggestion.title,
        &suggestion.command,
        suggestion.description.as_deref(),
        &left_by,
    )
    .await
    {
        Ok(true) => {
            tracing::info!(
                conversation_id,
                title = suggestion.title,
                left_by,
                "a session suggested a preview"
            );
            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });

            yaml(
                StatusCode::OK,
                &PreviewLeft {
                    title: suggestion.title,
                },
            )
        }
        Ok(false) => refused(
            conversation_id,
            "this Conversation has gone, so there is nowhere to suggest a preview".to_owned(),
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "suggesting a preview failed");

            yaml(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiError::new("Verkstead could not write the preview down, so run it again"),
            )
        }
    }
}

/// `POST /conversations/{conversation}/api/v1/previews/retire` — the session
/// running in this Conversation takes back the Preview whose title is the
/// body.
///
/// Under the rule suggesting one is under, plus one refusal of its own: a
/// title that does not stand, said by name. 204 once it is gone.
pub(crate) async fn retire(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
    title: String,
) -> Response {
    let title = title.trim();

    if let Err(refusal) = building(&state, conversation_id, "retire").await {
        return refusal;
    }

    match store::retire_preview(&state.pool, conversation_id, title).await {
        Ok(true) => {
            tracing::info!(conversation_id, title, "a session retired a preview");
            state.nudges.announce(Nudge::Conversation {
                conversation: conversation_id,
            });

            // And any terminal opened for it is an ordinary one from here,
            // called as any tab is (ADR-0024).
            if state.terminals.retired(conversation_id, title) {
                state.nudges.announce(Nudge::Terminals {
                    conversation: conversation_id,
                });
            }

            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => refused(
            conversation_id,
            format!(
                "no preview called {title:?} stands on this Conversation, so there is nothing \
                 to retire: retire one by the title it was suggested under"
            ),
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "retiring a preview failed");

            yaml(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiError::new("Verkstead could not retire the preview, so run it again"),
            )
        }
    }
}

/// What the building session running here was sent for, or the refusal saying
/// there is none — `doing` being the verb the refusal is about.
async fn building(
    state: &AppState,
    conversation_id: i64,
    doing: &str,
) -> Result<Building, Response> {
    let Some(event_id) = state.sessions.writing(conversation_id) else {
        return Err(refused(
            conversation_id,
            format!(
                "there is no session running in this Conversation, so there is nobody here to \
                 {doing} a preview"
            ),
        ));
    };

    crate::done::building(state, conversation_id, event_id)
        .await
        .ok_or_else(|| {
            refused(
                conversation_id,
                format!(
                    "only a building session may {doing} a preview — {MAY} — and this session \
                     was sent for something else"
                ),
            )
        })
}

fn refused(conversation_id: i64, why: String) -> Response {
    tracing::info!(conversation_id, why, "a session's preview was refused");

    yaml(StatusCode::CONFLICT, &ApiError::new(why))
}
