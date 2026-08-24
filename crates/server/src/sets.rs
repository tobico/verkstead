//! The Question Set endpoint: an agent's YAML comes in, an id goes back, and the
//! Set is on the Timeline of the Conversation its session is running for.
//!
//! Which Conversation is in the path, because it is in the base URL the sandbox
//! was given (see [`crate::sandbox::Reachable`]) — so the CLI attributes every
//! Set explicitly without knowing it is doing so, and nothing is inferred from
//! the project or the branch it derived. Two Conversations grilling one Repo
//! would be indistinguishable by either of those.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;
use verkstead_schema::{ApiError, Nudge, QuestionSet};

use crate::reply::yaml;
use crate::{AppState, store};

/// Which of the two kinds of ask the session is making, as the query string
/// carries it: `?deferred=true` for a Deferred Ask, and nothing at all for the
/// blocking one every ask was until now.
///
/// A query parameter rather than a field of the Set, because it is not part of
/// what was asked: the body is the agent's own words, kept as they were
/// written, and this is how the CLI was run. It also means an older CLI, which
/// says nothing, keeps asking exactly as it did.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Asking {
    deferred: bool,
}

impl Asking {
    fn kind(&self) -> store::Ask {
        match self.deferred {
            true => store::Ask::Deferred,
            false => store::Ask::Blocking,
        }
    }
}

/// `POST /conversations/{conversation}/api/v1/sets` — parse, validate, put it on
/// that Conversation's Timeline, and answer with the id the waiting agent will
/// poll on.
///
/// Malformed YAML is a 400; a well-formed Set that breaks the question grammar
/// is a 422 listing every violation, each naming the Question it belongs to. A
/// Conversation that is not there is a 404: there is nowhere for the Set to land
/// and nobody who would ever see it.
///
/// A Deferred Ask takes this same path and is answered the same way, id and all
/// — what differs is that nobody opens a wait on the id. So it lands on the
/// Timeline, leaves the Conversation *blocked on you* and notifies the human's
/// devices exactly as a blocking one does: both are something to answer, and the
/// human is not the one who is waiting.
pub(crate) async fn create_set(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
    Query(asking): Query<Asking>,
    body: String,
) -> Response {
    let set = match QuestionSet::from_yaml(&body) {
        Ok(set) => set,
        Err(error) => {
            return yaml(
                StatusCode::BAD_REQUEST,
                &ApiError::new(format!("the Question Set is not well-formed: {error}")),
            );
        }
    };

    if let Err(invalid) = set.validate() {
        return yaml(
            StatusCode::UNPROCESSABLE_ENTITY,
            &ApiError::with_violations(
                "the Question Set breaks the question grammar",
                invalid.violations,
            ),
        );
    }

    match store::ask(&state.pool, conversation_id, &set, asking.kind()).await {
        Ok(Some(created)) => {
            // Behind the answer, never in front of it: the agent hears that its
            // Set is stored the moment it is, and a push service that cannot be
            // reached costs a notification rather than the Set.
            crate::push::announce(&state.pool, created.id, &set);

            // And the pages that are already open, which hear it here rather
            // than the long way round through a push service. This is what puts
            // the Set in front of a human watching the Timeline it just landed
            // on.
            state.nudges.announce(Nudge::Set {
                conversation: conversation_id,
            });

            yaml(StatusCode::CREATED, &created)
        }
        Ok(None) => yaml(
            StatusCode::NOT_FOUND,
            &ApiError::new(format!(
                "there is no Conversation {conversation_id} to ask from"
            )),
        ),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "storing a Question Set failed");
            yaml(
                StatusCode::INTERNAL_SERVER_ERROR,
                &ApiError::new("the Question Set could not be stored"),
            )
        }
    }
}
