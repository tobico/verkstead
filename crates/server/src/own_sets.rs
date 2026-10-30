//! Settling the Question Sets Verkstead asks itself (ADR-0029, *Verkstead asks
//! with a Question Set of its own*).
//!
//! An agent's Set is settled by the agent: the Response goes back to whoever is
//! waiting on it, and what it means is theirs to act on. One of Verkstead's has
//! nobody waiting on it, so the Response is acted on here, the moment it is
//! taken — from the viewer and from the agent-facing endpoint alike, which are
//! both ways a Response is stored. What each one asked about is the record's,
//! kept beside the Set — see [`store::OwnAsking`].

use verkstead_schema::Response;

use crate::{AppState, store};

/// Act on `response`, just taken for Set `set_id`, where the Set is Verkstead's
/// own. An agent's Set is left alone.
pub(crate) async fn settle(state: &AppState, set_id: i64, response: &Response) {
    let asking = match store::own_asking(&state.pool, set_id).await {
        Ok(Some(asking)) => asking,
        Ok(None) => return,
        Err(error) => {
            tracing::error!(error = ?error, set_id, "reading what a Set of Verkstead's asked about failed, so its answer was not acted on");
            return;
        }
    };

    let conversation_id = match store::asked_from(&state.pool, set_id).await {
        Ok(Some(conversation_id)) => conversation_id,
        Ok(None) => {
            tracing::error!(
                set_id,
                "a Set of Verkstead's is on no Timeline, so its answer was not acted on"
            );
            return;
        }
        Err(error) => {
            tracing::error!(error = ?error, set_id, "reading where a Set of Verkstead's was asked failed, so its answer was not acted on");
            return;
        }
    };

    match &asking {
        store::OwnAsking::Placing { .. } => {
            crate::placing::answered(state, conversation_id, &asking, response).await
        }
    }
}
