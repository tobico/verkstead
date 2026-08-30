//! The Question Set endpoint: an agent's YAML comes in, an id goes back, and the
//! Set is on the Timeline of the Conversation its session is running for.
//!
//! Which Conversation is in the path, because it is in the base URL the sandbox
//! was given (see [`crate::sandbox::Reachable`]) — so the CLI attributes every
//! Set explicitly without knowing it is doing so, and nothing is inferred from
//! the project or the branch it derived. Two Conversations grilling one Repo
//! would be indistinguishable by either of those.
//!
//! Knowing the Conversation is also what lets the Diff be read here rather than
//! sent: the Worktrees it names are on this host, so their uncommitted changes
//! are composed as the Set arrives, one block per repository — see
//! [`crate::diffs`].
//!
//! And the other end of the same subject: closing the Sets a Conversation has
//! left open, which is what [`lock`] below is. Two things reach for it — a
//! grilling relaunched over a session that died, and a Conversation being closed
//! — and what they differ over is which Sets they mean rather than what locking
//! one does, so [`Open`] is the whole of the difference between them.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;
use verkstead_schema::{ApiError, Nudge, QuestionSet};

use crate::reply::yaml;
use crate::{AppState, store};

/// Whether the session says nobody is to idle on this Set, as the query string
/// carries it: `?deferred=true` for a Deferred Ask, and nothing at all for the
/// ordinary one.
///
/// A query parameter rather than a field of the Set, because it is not part of
/// what was asked: the body is the agent's own words, kept as they were
/// written, and this is how the CLI was run. It also means an older CLI, which
/// says nothing, keeps asking exactly as it did.
///
/// The whole of what the agent says about how it asked, at that. Whether an
/// ordinary ask is waited on or stored is the backend's fact rather than the
/// agent's — see [`Asking::kind`].
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct Asking {
    deferred: bool,
}

impl Asking {
    /// Which of the three kinds this ask is: what the agent said, read against
    /// the channel its backend asks on.
    ///
    /// `--deferred` means an ask nobody is idling on, on every backend — it is
    /// the agent saying it will carry straight on, and no backend makes that
    /// untrue. What is left is the ordinary ask, and that is the backend's to
    /// answer: a session that can hold `verkstead ask` open for hours idles on
    /// it, and one whose shell tool yields after seconds has the Set stored and
    /// is nudged when the Response lands (ADR-0011).
    fn kind(&self, channel: store::Channel) -> store::Ask {
        match (self.deferred, channel) {
            (true, _) => store::Ask::Deferred,
            (false, store::Channel::Blocking) => store::Ask::Blocking,
            (false, store::Channel::StoreAndNudge) => store::Ask::StoreAndNudge,
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
/// A stored ask takes this same path and is answered the same way, id and all —
/// what differs is that nobody opens a wait on the id, and the reply says so.
/// So it lands on the Timeline, leaves the Conversation *blocked on you* and
/// notifies the human's devices exactly as a blocking one does: both are
/// something to answer, and the human is not the one who is waiting.
///
/// Which of the two an ordinary ask is depends on the backend that sent it and
/// not on anything in the request — see [`Asking::kind`], and
/// [`crate::sessions::Sessions::channel`] for where the backend is known.
pub(crate) async fn create_set(
    State(state): State<AppState>,
    Path(conversation_id): Path<i64>,
    Query(asking): Query<Asking>,
    body: String,
) -> Response {
    let mut set = match QuestionSet::from_yaml(&body) {
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

    // Whatever the Set claims its Diff is, the Conversation's Worktrees are the
    // authority — and this is where that read happens now, close enough to them
    // to do it. One block per repository the work may be written in, and the
    // single field a Set used to carry left empty: it is what the Sets stored
    // before the Diff became a list are read back through, and not somewhere for
    // a claim of the agent's to survive. See [`crate::diffs`].
    set.diff = None;
    set.diffs = crate::diffs::compose(&state.pool, conversation_id).await;

    // How this session asks, which is the backend's fact and so the server's to
    // supply: the CLI asked the same way it would have on any of them.
    let kind = asking.kind(state.sessions.channel(conversation_id));

    match store::ask(&state.pool, conversation_id, &set, kind).await {
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

/// Which of the Sets left open a caller means.
///
/// The two readings differ over a **Deferred Ask**, which is a question nobody
/// is waiting on the answer to — see [`crate::deferrals`]. A session going away
/// takes nothing from one, so a relaunch leaves it standing; a Conversation
/// closing takes away every session there will ever be, so nothing is left to
/// fold the answer into and the question is over too.
///
/// A store-and-nudge ask is on the narrow side of that line with the blocking
/// ones, stored though it is: the session idling on one has gone in both cases
/// this is asked in, and a stored Set nobody is coming back for is a question
/// with no reader exactly as a blocking one is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Open {
    /// The ones a session was idling on — see [`store::Ask::idled`], which is
    /// the one place that question is answered.
    Idled,

    /// Every Set the human could still answer, whichever kind of ask it was.
    Either,
}

/// The Sets on `timeline` that are still open — no Response and no lock — of
/// the kinds `wanted` asks for.
pub(crate) fn open(timeline: &[store::TimelineEvent], wanted: Open) -> Vec<i64> {
    timeline
        .iter()
        .filter_map(|event| match &event.event {
            store::Event::QuestionSet(asked) => (asked.settlement.is_none()
                && (wanted == Open::Either || asked.ask.idled()))
            .then_some(asked.set_id),
            _ => None,
        })
        .collect()
}

/// Lock every Set in `sets` unanswered, so that nothing is left for the human to
/// answer into, and tell the pages they were open on.
///
/// An open Set is a question with a reader, and the reader has gone: the badge
/// still says *blocked on you*, the Set still takes an Answer, and what the
/// human writes goes nowhere. Locking unanswered is what that Set has always
/// meant — see [`store::lock_set`] — and this is Verkstead reaching for it on
/// their behalf, because it knows something they cannot see. `because` is what
/// it knows, and it goes in the log beside the Set.
///
/// Nothing is refused for. A Set that will not lock is a Set the human can lock
/// themselves from the page it is on, and stopping over one would leave whatever
/// asked for this half done.
pub(crate) async fn lock(
    state: &AppState,
    conversation_id: i64,
    sets: &[i64],
    because: &'static str,
) {
    let mut locked = false;

    for &set_id in sets {
        match store::lock_set(&state.pool, &state.settlements, set_id).await {
            Ok(store::Locking::Locked(_)) => {
                locked = true;

                tracing::info!(
                    conversation_id,
                    set_id,
                    because,
                    "a Question Set left open was locked unanswered"
                );
            }
            Ok(other) => tracing::info!(
                conversation_id,
                set_id,
                because,
                outcome = ?other,
                "a Question Set left open was settled before the locking reached it",
            ),
            Err(error) => {
                tracing::error!(error = ?error, conversation_id, set_id, because, "locking a Question Set left open failed");
            }
        }
    }

    // The page the human is looking at is the page the Set was open on, and what
    // has just changed there is that it no longer is. Only where something did
    // change: a nudge is every open page going back to the store.
    if locked {
        state.nudges.announce(Nudge::Set {
            conversation: conversation_id,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The barest Set there is: what these are about is the row around it.
    const ASKED: &str = r#"
title: How the limiter counts
questions:
  - label: Q1
    text: Per key or per address?
    options:
      - n: 1
        text: Per key
"#;

    /// One Set on a Timeline, asked the given way and settled however the caller
    /// says — or still waiting on the human, where they say nothing.
    fn on_timeline(
        set_id: i64,
        ask: store::Ask,
        settlement: Option<store::Settlement>,
    ) -> store::TimelineEvent {
        store::TimelineEvent {
            id: set_id,
            at: "2026-08-28T12:00:00Z".to_owned(),
            event: store::Event::QuestionSet(Box::new(store::SetOnTimeline {
                set_id,
                set: store::Asked::Set(
                    QuestionSet::from_yaml(ASKED).expect("the example Set parses"),
                ),
                settlement,
                ask,
            })),
        }
    }

    fn locked(set_id: i64) -> store::Settlement {
        store::Settlement::LockedUnanswered(store::SetLocked {
            set_id,
            locked_at: "2026-08-28T12:06:00Z".to_owned(),
        })
    }

    /// A Set that has settled — either way — is not open, whichever reading is
    /// asked for. There is nothing left to lock about one.
    #[test]
    fn a_settled_set_is_open_to_neither_reading() {
        let timeline = vec![
            on_timeline(11, store::Ask::Blocking, Some(locked(11))),
            on_timeline(12, store::Ask::Deferred, Some(locked(12))),
        ];

        assert!(open(&timeline, Open::Idled).is_empty());
        assert!(open(&timeline, Open::Either).is_empty());
    }

    /// And the Deferred Ask is the whole of what the two readings differ over: a
    /// relaunch leaves one standing for the session after it, and a Conversation
    /// closing has no session after it to leave one for.
    ///
    /// A store-and-nudge ask is not one of them, stored though it is: the
    /// session idling on it has gone either way, so it locks with the blocking
    /// ones.
    #[test]
    fn a_deferred_ask_is_open_only_to_the_wider_reading() {
        let timeline = vec![
            on_timeline(11, store::Ask::Blocking, None),
            on_timeline(12, store::Ask::Deferred, None),
            on_timeline(13, store::Ask::StoreAndNudge, None),
        ];

        assert_eq!(open(&timeline, Open::Idled), vec![11, 13]);
        assert_eq!(open(&timeline, Open::Either), vec![11, 12, 13]);
    }

    /// The ordinary ask is the backend's to answer, and `--deferred` is the
    /// agent's on every backend.
    ///
    /// Which is the whole of the mapping: nothing the CLI sends says which
    /// channel it is on, and nothing about the backend makes an ask the agent
    /// said it would not wait for into one somebody is idling on.
    #[test]
    fn the_backend_decides_an_ordinary_ask_and_the_agent_decides_a_deferred_one() {
        let ordinary = Asking { deferred: false };
        let deferred = Asking { deferred: true };

        assert_eq!(
            ordinary.kind(store::Channel::Blocking),
            store::Ask::Blocking
        );
        assert_eq!(
            ordinary.kind(store::Channel::StoreAndNudge),
            store::Ask::StoreAndNudge
        );

        assert_eq!(
            deferred.kind(store::Channel::Blocking),
            store::Ask::Deferred
        );
        assert_eq!(
            deferred.kind(store::Channel::StoreAndNudge),
            store::Ask::Deferred,
            "a backend that stores every ask does not make one nobody is idling \
             on into one somebody is",
        );
    }
}
