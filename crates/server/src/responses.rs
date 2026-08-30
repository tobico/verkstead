//! The Response endpoints: the human's reply in, and the agent's Answers out —
//! whether it held a long-poll open for them or came back for them in one poll,
//! which is the same request asked to hold for nothing.
//!
//! Both are reached through the Conversation the Set was asked from, because
//! that is what the session's base URL says — see [`crate::sets`]. A Set id that
//! belongs to another Conversation names nothing here: a scope that was only
//! written into the path and never read would be no scope at all.

use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response as HttpResponse};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::{Instant, timeout};
use verkstead_schema::{ApiError, Nudge, Response};
use verkstead_store::{Settlement, Submission, WaitHeld};

use crate::nudge::Nudges;
use crate::reply::yaml;
use crate::{AppState, MAX_HOLD, store};

/// `POST /conversations/{conversation}/api/v1/sets/{id}/response` — take the
/// human's reply, check it resolves the Set, store it, and wake whoever is
/// waiting.
///
/// Malformed YAML is a 400; a Response that leaves a question unaccounted for
/// is a 422 naming it. A Set is answered once: a second Response is a 409 and
/// the first one stands. A Set the human locked unanswered is closed for good,
/// so a Response to one is a 410.
pub(crate) async fn submit_response(
    State(state): State<AppState>,
    Path((conversation_id, id)): Path<(i64, i64)>,
    body: String,
) -> HttpResponse {
    let response = match Response::from_yaml(&body) {
        Ok(response) => response,
        Err(error) => {
            return yaml(
                StatusCode::BAD_REQUEST,
                &ApiError::new(format!("the Response is not well-formed: {error}")),
            );
        }
    };

    match asked_from(&state, conversation_id, id).await {
        Ok(true) => {}
        Ok(false) => return not_found(id),
        Err(refusal) => return refusal,
    }

    match store::submit_response(&state.pool, &state.settlements, id, &response).await {
        Ok(Submission::Accepted(taken)) => {
            crate::conversations::settle_a_proposal(&state, id, taken.proposed).await;

            // The acceptance and nothing about the move: what a waiting agent
            // came for is that its Set is answered, and the Conversation moving
            // on is the Timeline's business rather than the agent contract's.
            yaml(StatusCode::CREATED, &taken.accepted)
        }
        Ok(Submission::NoSuchSet) => not_found(id),
        Ok(Submission::Locked) => gone(id),
        Ok(Submission::Invalid(invalid)) => yaml(
            StatusCode::UNPROCESSABLE_ENTITY,
            &ApiError::with_violations(
                "the Response does not resolve the Question Set",
                invalid.violations,
            ),
        ),
        Ok(Submission::AlreadyAnswered) => yaml(
            StatusCode::CONFLICT,
            &ApiError::new(format!("Question Set {id} has already been answered")),
        ),
        Err(error) => {
            tracing::error!(error = ?error, set_id = id, "taking a Response failed");
            unavailable("the Response could not be taken")
        }
    }
}

/// How long a wait is held open when the client does not say.
const DEFAULT_HOLD: Duration = Duration::from_secs(30);

/// What the client asks for when it opens a wait.
#[derive(Debug, Deserialize)]
pub(crate) struct Wait {
    /// How many seconds the client is willing to have the request held open,
    /// clamped to [`MAX_HOLD`]. `0` makes it a plain poll.
    hold: Option<u64>,
}

/// `GET /conversations/{conversation}/api/v1/sets/{id}/response` — hand the
/// Response to the waiting agent.
///
/// Answers straight away if the Set has been answered already; otherwise holds
/// the connection until it is, or until the hold window closes and the reply
/// is a bare 204 meaning "nothing yet, come back". There is no expiry on the
/// waiting itself: the client owns retry, so it simply opens another wait.
///
/// A Set the human locked unanswered ends the wait with a 410 instead: nothing
/// is ever coming, and the client is told so rather than left polling a Set
/// nobody will answer.
///
/// Handing the Response over is a delivery either way, and is written down as
/// one — see [`delivered`], which is what keeps a session's own Answers from
/// also arriving under the next session's prompt.
pub(crate) async fn wait_for_response(
    State(state): State<AppState>,
    Path((conversation_id, id)): Path<(i64, i64)>,
    Query(wait): Query<Wait>,
) -> HttpResponse {
    let hold = wait
        .hold
        .map_or(DEFAULT_HOLD, Duration::from_secs)
        .min(MAX_HOLD);

    // Subscribe before the first read, so a Set settled between the two wakes
    // this wait instead of slipping past it.
    let mut settlements = state.settlements.subscribe();

    // Which also answers whether the Set is there at all: a Set is on exactly
    // one Timeline, so being on this Conversation's is the whole question.
    match asked_from(&state, conversation_id, id).await {
        Ok(true) => {}
        Ok(false) => return not_found(id),
        Err(refusal) => return refusal,
    }

    // Taken once the Set is known to exist, so a 404 leaves no trace in the
    // registry, and held for exactly as long as this future lives: a client that
    // vanishes mid-hold has its future dropped rather than returned, and the
    // guard is what both endings have in common. Display only — nothing below
    // consults it, and no Set is withdrawn for want of one.
    let _held = Watched::over(&state, conversation_id, id);

    let deadline = Instant::now() + hold;

    loop {
        match store::settlement(&state.pool, id).await {
            Ok(Some(Settlement::Answered(stored))) => {
                // Handing it over is the delivery, whether this was a wait held
                // open or a fetch that came back for it.
                delivered(&state, id).await;
                return yaml(StatusCode::OK, &stored.response);
            }
            Ok(Some(Settlement::LockedUnanswered(_))) => return gone(id),
            Ok(None) => {}
            Err(error) => {
                tracing::error!(error = ?error, set_id = id, "loading a Response failed");
                return unavailable("the Response could not be read");
            }
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return StatusCode::NO_CONTENT.into_response();
        }

        if !matches!(
            timeout(remaining, settled(&mut settlements, id)).await,
            Ok(true)
        ) {
            return StatusCode::NO_CONTENT.into_response();
        }
    }
}

/// A wait held on a Set, with the open pages told at both ends of it.
///
/// The badge a waiting Set carries is read off the registry underneath, and the
/// two moments it can move are an agent taking a slot and the last slot going —
/// so those are the two moments a page is told to look. Nothing durable has
/// happened at either, which is why this is a kind of its own: it used to be
/// carried by the viewer's ten-second poll, and the poll is gone (ADR-0009).
///
/// The far end is a `Drop`, for the same reason the slot's release is one: a
/// client that vanishes mid-hold has this future dropped rather than returned,
/// and dropping is the one thing every ending has in common.
///
/// A badge that reads *waiting* for a grace period after the last release is
/// still the registry's business, so the Nudge sent as this goes is one the
/// verdict may not have moved on yet — a page reads back a badge that has not
/// changed, which costs one small request. What it buys is the reconnect a
/// second later being seen at once.
struct Watched {
    /// The slot itself, released as this is dropped.
    _held: WaitHeld,

    /// Kept rather than reached for through the state, because `Drop` runs
    /// wherever the future was dropped and has nothing else to hand.
    nudges: Nudges,
    conversation_id: i64,
}

impl Watched {
    /// Take a slot on `set_id` and say so.
    fn over(state: &AppState, conversation_id: i64, set_id: i64) -> Self {
        let watched = Self {
            _held: state.waits.hold(set_id),
            nudges: state.nudges.clone(),
            conversation_id,
        };
        watched.announce();
        watched
    }

    fn announce(&self) {
        self.nudges.announce(Nudge::Liveness {
            conversation: self.conversation_id,
        });
    }
}

impl Drop for Watched {
    fn drop(&mut self) {
        self.announce();
    }
}

/// Record that a session has been handed Set `id`'s Answers, so the folding
/// rule passes over it.
///
/// A fetch is a delivery: the Answers reach the session that asked or a later
/// session's prompt, never both — see [`crate::deferrals`]. Written here, in the
/// request that hands the Response over, rather than by anything the CLI says
/// afterwards, so a session that read its Answers and then died still counts as
/// having read them.
///
/// Nothing to write for a Blocking Ask, which has no folding record at all, and
/// nothing is refused for a write that fails: the cost of one is the human's
/// Answers arriving twice, which is worse than losing the Response they were.
async fn delivered(state: &AppState, id: i64) {
    if let Err(error) = store::record_folded(&state.pool, &[id]).await {
        tracing::error!(error = ?error, set_id = id, "recording a fetched Question Set as folded failed");
    }
}

/// Wait until this Set is settled, one way or the other. `false` when the
/// channel is gone, which only happens as the server itself does.
async fn settled(settlements: &mut broadcast::Receiver<store::SettledSet>, id: i64) -> bool {
    loop {
        match settlements.recv().await {
            Ok(settled) if settled.set_id == id => return true,
            Ok(_) => continue,
            // Overtaken by a burst of settlements: ours may have been among
            // them, so go back and look at the store.
            Err(RecvError::Lagged(_)) => return true,
            Err(RecvError::Closed) => return false,
        }
    }
}

/// Whether this Set was asked from this Conversation, or a refusal to answer
/// with where the store could not say.
///
/// `Ok(false)` is both "no such Set" and "not this Conversation's", which the
/// endpoints answer for the same way and should: a session reaches Verkstead
/// through its own Conversation alone, so another one's Set is not a Set it may
/// be told anything about, including that it exists.
async fn asked_from(state: &AppState, conversation_id: i64, id: i64) -> Result<bool, HttpResponse> {
    store::set_asked_from(&state.pool, conversation_id, id)
        .await
        .map_err(|error| {
            tracing::error!(
                error = ?error,
                conversation_id,
                set_id = id,
                "looking for a Question Set failed"
            );
            unavailable("the Question Set could not be read")
        })
}

fn not_found(id: i64) -> HttpResponse {
    yaml(
        StatusCode::NOT_FOUND,
        &ApiError::new(format!("there is no Question Set {id}")),
    )
}

/// The Set was locked unanswered: it is closed, and no amount of retrying will
/// change that. The CLI treats this as fatal.
fn gone(id: i64) -> HttpResponse {
    yaml(
        StatusCode::GONE,
        &ApiError::new(format!(
            "Question Set {id} was locked unanswered, so it is closed"
        )),
    )
}

fn unavailable(message: &str) -> HttpResponse {
    yaml(StatusCode::INTERNAL_SERVER_ERROR, &ApiError::new(message))
}
