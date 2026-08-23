//! The Nudge: telling the open viewer pages that the pending world moved.
//!
//! A Nudge is notify-only (ADR-0009). It names a kind — a Transcript grew, a
//! commit landed, a Set settled — and, where the change belongs to one, the
//! Conversation it happened in. It never carries what changed: the page is told
//! to look, and looking is an ordinary HTTP read. Nothing here has to be got
//! right for correctness either; a Nudge that never lands costs latency, and
//! the page's catch-up on reconnect collects the change anyway.
//!
//! The changes reach this stream over two channels rather than one. A settlement
//! — a Response taken, or a Set closed unanswered — is already announced on
//! [`Settlements`](verkstead_store::Settlements) from inside the store, on the
//! single path the browser's submit and the agent's both take, so listening to it
//! here is what makes it impossible to Nudge about a settlement from one
//! namespace and silently not from the other. Everything else that moves — a Set
//! arriving, a session printing another line of its Capture — is announced on
//! [`Nudges`], which is the channel for what the store has no reason to know has
//! happened. Both come out of the merge as the same typed [`Nudge`].
//!
//! Liveness stays out of it, and so stays with the poll: the
//! waiting/disconnected verdict cycles with the agent's long-poll rather than
//! changing at clean moments, so there is no moment here to broadcast.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::Sse;
use axum::response::sse::{Event, KeepAlive};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use verkstead_schema::Nudge;
use verkstead_store::SettledSet;

use crate::AppState;

/// How much may happen before a page that is behind is told it is. Falling
/// behind costs precision rather than correctness: a page that missed a burst is
/// told it missed one, and reads back everything it is showing.
const NUDGE_BACKLOG: usize = 16;

/// How often the stream says something into the quiet. Anything between the
/// page and the server may close a connection that has gone silent, and a
/// reconnect is the slow way to find out the world moved — this is what keeps
/// the stream open rather than reopened.
const KEEP_ALIVE: Duration = Duration::from_secs(15);

/// Word that the world moved in a way the store has no reason to announce: a
/// Question Set arriving, a session saying something.
///
/// The counterpart to the store's `Settlements`, for the changes that have
/// nowhere else to come from. Nothing waits on a Set arriving and nothing waits
/// on a Capture growing, so until the viewer wanted to hear about them there
/// was nobody to tell.
#[derive(Debug, Clone)]
pub(crate) struct Nudges(broadcast::Sender<Nudge>);

impl Nudges {
    pub(crate) fn new() -> Self {
        let (moves, _) = broadcast::channel(NUDGE_BACKLOG);
        Self(moves)
    }

    /// Tell the open pages what moved. A send error means none are open, which
    /// is the ordinary case — there is usually no browser pointed at this at
    /// all.
    ///
    /// The kind is not optional and has no default: what a caller knows about
    /// what it just changed is knowledge that exists nowhere else, and a Nudge
    /// that shrugged would put every page back to reading everything.
    pub(crate) fn announce(&self, moved: Nudge) {
        let _ = self.0.send(moved);
    }

    fn subscribe(&self) -> broadcast::Receiver<Nudge> {
        self.0.subscribe()
    }
}

/// `GET /api/ui/nudges` — the stream an open page listens on.
pub(crate) async fn nudges(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Subscribed to before the response goes back, so a change landing while
    // the page is still opening the stream is one it hears rather than one that
    // slips past it.
    let moved = BroadcastStream::new(state.nudges.subscribe());
    let settled = BroadcastStream::new(state.settlements.subscribe());

    // A page that fell behind is told nothing narrower, because there is nothing
    // narrow enough to tell it: what it missed is a burst it saw the middle of
    // none of. The stream ends instead, the browser reconnects, and the
    // reconnect is what reads the world back whole — the same catch-up a
    // dropped connection gets, which is the one this already relies on.
    let nudges = moved
        .merge(settled.map(|settled| settled.map(settlement)))
        .take_while(Result::is_ok)
        .map(|moved| {
            Ok(frame(
                moved.expect("a Nudge that was not one ended the stream above"),
            ))
        });

    Sse::new(nudges).keep_alive(KeepAlive::new().interval(KEEP_ALIVE))
}

/// What a settlement off the store's channel says on this one.
///
/// A Set that settled without a Conversation behind it is a record that has been
/// got at, rather than something a Set can be: every Set is asked from a
/// Conversation, on one path, in one transaction. The list of Conversations is
/// the widest thing there is to point a page at when it has happened anyway.
fn settlement(settled: SettledSet) -> Nudge {
    match settled.conversation_id {
        Some(conversation) => Nudge::Set { conversation },
        None => {
            tracing::error!(
                set_id = settled.set_id,
                "a Question Set settled that is on no Conversation's Timeline",
            );
            Nudge::Conversations
        }
    }
}

/// One Nudge on the wire.
///
/// The event stays named, so that whatever else may one day come down this
/// stream is not mistaken for a Nudge by a page too old to know about it. What
/// it says is in the data now, as the JSON the viewer's `Nudge` type is
/// generated from.
fn frame(moved: Nudge) -> Event {
    Event::default().event("nudge").json_data(moved).expect(
        "a Nudge is a tagged enum of integers and unit variants, which serialises without fail",
    )
}
