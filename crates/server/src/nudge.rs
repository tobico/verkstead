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
//! Liveness is in it now that the viewer's poll is gone. The
//! waiting/disconnected verdict used to cycle with that poll; what it cycles
//! with here is the agent's long-poll itself, announced as it is taken up and
//! as it is let go — see [`crate::responses`], which is what holds it.
//!
//! **And a member's news comes down this stream too.** A page reaches a
//! Conversation of a member's through the device it opened (ADR-0020, *The
//! opened device relays*), so the news of one has to arrive on the stream that
//! page is already listening to: the hub holds a Nudge stream to each of its
//! members and announces what comes down one here, under the Device Id it came
//! from — see [`crate::relaying::freshness`], and [`Nudged`], which is the
//! device and the Nudge together. A local Nudge carries no device and is the
//! frame it always was.
//!
//! **What goes over the Peer Listener is this device's own news alone** — see
//! [`nudges`]. That namespace is mounted twice and this is the one endpoint in it
//! that answers the two listeners differently, because a member re-announcing
//! what a *third* device said would be saying it was this device's: in a cluster
//! everybody holds a stream to everybody, so the news of C reaches every member
//! from C itself.

use std::convert::Infallible;
use std::time::Duration;

use axum::Extension;
use axum::extract::State;
use axum::response::Sse;
use axum::response::sse::{Event, KeepAlive};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use verkstead_schema::{Nudge, Nudged};
use verkstead_store::SettledSet;

use crate::AppState;
use crate::peer::workbench::OverTheLink;

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
pub struct Nudges(broadcast::Sender<Nudged>);

impl Nudges {
    pub fn new() -> Self {
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
    pub fn announce(&self, moved: Nudge) {
        let _ = self.0.send(Nudged::here(moved));
    }

    /// And the same about a **member's** world rather than this device's: what
    /// came down the Nudge stream this device holds to `device`, said again here
    /// under the Device Id it came from.
    ///
    /// The kind is carried through untouched, because it is the member's own
    /// account of what moved and this device has nothing to add to it: what the
    /// device does is say *whose* it is, which is the one thing the far end
    /// could not say — a Nudge is written by a Verkstead that has no idea who is
    /// reading it.
    ///
    /// Only ever called by [`crate::relaying::freshness`]. Everything else in
    /// this tree is telling the pages about work this device is doing itself,
    /// and has no device to name.
    pub(crate) fn announce_of(&self, device: &str, moved: Nudge) {
        let _ = self.0.send(Nudged::of(device, moved));
    }

    /// Listen to what is announced. The stream is one listener; a test that
    /// wants to know whether a caller told the pages anything is another.
    pub(crate) fn subscribe(&self) -> broadcast::Receiver<Nudged> {
        self.0.subscribe()
    }
}

impl Default for Nudges {
    fn default() -> Self {
        Nudges::new()
    }
}

/// `GET /api/ui/nudges` — the stream an open page listens on, and the one a
/// member holds to this device.
///
/// **The one endpoint in this namespace that answers the two listeners
/// differently**, and the difference is one filter: what goes over the Peer
/// Listener is this device's own news, where a browser's stream carries that and
/// every member's besides. A device that passed on what a *third* device told it
/// would be saying a stream's news was its own, and the reader would re-announce
/// it under the wrong Device Id — while there is nothing to pass on in the first
/// place, a cluster being a membership every device holds the whole of: the news
/// of C reaches every member from C's own stream.
///
/// Which listener this is, is [`OverTheLink`] — put beside the request by the
/// router a member reaches, and absent from the one a browser does.
pub(crate) async fn nudges(
    State(state): State<AppState>,
    over_the_link: Option<Extension<OverTheLink>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let its_own_news = over_the_link.is_some();

    // Subscribed to before the response goes back, so a change landing while
    // the page is still opening the stream is one it hears rather than one that
    // slips past it.
    let moved = BroadcastStream::new(state.nudges.subscribe()).filter(move |moved| match moved {
        Ok(moved) => !its_own_news || moved.device.is_none(),
        // Kept, because what it says is that this reader fell behind, and
        // the take-while below is what reads it — see there.
        Err(_) => true,
    });
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
/// This device's own, always: a settlement is a Response landing in this store,
/// which is exactly as true of one a member relayed in as of one the browser
/// here submitted — the record that moved is this device's either way.
///
/// A Set that settled without a Conversation behind it is a record that has been
/// got at, rather than something a Set can be: every Set is asked from a
/// Conversation, on one path, in one transaction. The list of Conversations is
/// the widest thing there is to point a page at when it has happened anyway.
fn settlement(settled: SettledSet) -> Nudged {
    match settled.conversation_id {
        Some(conversation) => Nudged::here(Nudge::Set { conversation }),
        None => {
            tracing::error!(
                set_id = settled.set_id,
                "a Question Set settled that is on no Conversation's Timeline",
            );
            Nudged::here(Nudge::Conversations)
        }
    }
}

/// One Nudge on the wire.
///
/// The event stays named, so that whatever else may one day come down this
/// stream is not mistaken for a Nudge by a page too old to know about it. What
/// it says is in the data now, as the JSON the viewer's `Nudged` type is
/// generated from — the kind, the Conversation where the change belongs to one,
/// and the device where the news is a member's.
fn frame(moved: Nudged) -> Event {
    Event::default().event("nudge").json_data(moved).expect(
        "a Nudge is a tagged enum of integers and unit variants under an optional Device Id, \
         which serialises without fail",
    )
}
