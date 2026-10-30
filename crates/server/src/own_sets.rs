//! Settling the Question Sets Verkstead asks itself (ADR-0029, *Verkstead asks
//! with a Question Set of its own*).
//!
//! An agent's Set is settled by the agent: the Response goes back to whoever is
//! waiting on it, and what it means is theirs to act on. One of Verkstead's has
//! nobody waiting on it, so the Response is acted on here, the moment it is
//! taken — from the viewer and from the agent-facing endpoint alike, which are
//! both ways a Response is stored. What each one asked about is the record's,
//! kept beside the Set — see [`store::OwnAsking`].
//!
//! **An open Set of Verkstead's is the work waiting on the human**, exactly as
//! a session idling on an ask is, so it is held on the drivers register for as
//! long as it is open — see [`Holds`]. That is what takes it a place on the
//! server, what keeps the stall sweep from stopping it as *nothing is driving
//! it*, and what a Resume pressed meanwhile finds already driven.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;
use verkstead_schema::Response;

use crate::drivers::Driving;
use crate::{AppState, store};

/// The open Sets of Verkstead's own being waited on, each holding a driver
/// registration for its Conversation.
///
/// Keyed by the Set rather than the Conversation, so that a second launch
/// reaching the same open Set — a restart's resume, a second driver — finds it
/// held already rather than holding it twice. In-process, as every driver
/// registration is: a restart holds none, and the startup resume reaching the
/// launch again is what finds the Set and holds it again — see
/// [`crate::placing`].
#[derive(Clone, Default)]
pub(crate) struct Holds(Arc<Mutex<HashMap<i64, Driving>>>);

impl Holds {
    pub(crate) fn new() -> Holds {
        Holds::default()
    }

    fn holds(&self) -> std::sync::MutexGuard<'_, HashMap<i64, Driving>> {
        self.0.lock().expect("the own Sets' holds are not poisoned")
    }

    /// Let go of Set `set_id`'s registration, handing it to the caller.
    fn take(&self, set_id: i64) -> Option<Driving> {
        self.holds().remove(&set_id)
    }
}

/// Wait on Set `set_id` of Verkstead's, open on `conversation_id`: hold a
/// driver registration for the Conversation until it settles.
///
/// Answered, the registration is [`settle`]'s, which holds it across acting on
/// the answer so that a launch here takes the place over without a gap. Locked
/// unanswered, nothing is acted on and it is simply let go — heard off the
/// settlements, which every lock and every answer announces.
///
/// A Set already held is left as it is.
pub(crate) async fn wait(state: &AppState, conversation_id: i64, set_id: i64) {
    // Subscribed before the store is read, so a settle landing in between wakes
    // the watch rather than slipping past it.
    let mut settlements = state.settlements.subscribe();

    {
        let mut holds = state.own_holds.holds();

        if holds.contains_key(&set_id) {
            return;
        }

        holds.insert(set_id, state.drivers.driving(conversation_id));
    }

    // Settled before it was held — answered as a restart's resume reached it,
    // say — and nothing will announce it again.
    if !still_open(state, conversation_id, set_id).await {
        state.own_holds.take(set_id);
        return;
    }

    let state = state.clone();

    tokio::spawn(async move {
        loop {
            match settlements.recv().await {
                Ok(settled) if settled.set_id != set_id => continue,
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return,
            }

            match standing(&state, conversation_id, set_id).await {
                // Still open: an announcement missed for a backlog, read again.
                Some((None, false)) => continue,

                // Answered, and the registration is the answer's to let go.
                Some((Some(_), _)) => return,

                // Locked unanswered, or gone: nobody is waiting on it now.
                Some((None, true)) | None => {
                    state.own_holds.take(set_id);
                    return;
                }
            }
        }
    });
}

/// Whether Set `set_id` is neither answered nor locked.
async fn still_open(state: &AppState, conversation_id: i64, set_id: i64) -> bool {
    matches!(
        standing(state, conversation_id, set_id).await,
        Some((None, false))
    )
}

/// How Set `set_id` of Verkstead's stands: its Response, where it has one, and
/// whether it was locked. `None` where it cannot be read.
async fn standing(
    state: &AppState,
    conversation_id: i64,
    set_id: i64,
) -> Option<(Option<Response>, bool)> {
    match store::own_sets(&state.pool, conversation_id).await {
        Ok(own) => own
            .into_iter()
            .find(|own| own.set_id == set_id)
            .map(|own| (own.response, own.locked)),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, set_id, "reading how a Set of Verkstead's stands failed");
            None
        }
    }
}

/// Act on `response`, just taken for Set `set_id`, where the Set is Verkstead's
/// own. An agent's Set is left alone.
///
/// The Set's hold on the drivers register is handed to whatever acts on the
/// answer, and let go once it has: a launch here takes the place over from it,
/// and a move frees it.
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

    let held = state.own_holds.take(set_id);

    match &asking {
        store::OwnAsking::Placing { .. } => {
            crate::placing::answered(state, conversation_id, &asking, response, held).await
        }
    }
}
