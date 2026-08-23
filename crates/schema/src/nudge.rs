//! The Nudge: what the server tells an open viewer page has moved.
//!
//! Notify-only (ADR-0009). A Nudge names a kind and, where the change belongs to
//! one, the Conversation it happened in — never what changed. Data rides no
//! event: HTTP stays the single path for content, rendering and types, and a
//! Nudge is the word that a fetch is worth making.
//!
//! The kinds are the server's vocabulary for what moved, not the viewer's cache
//! layout. Which queries a kind stands for is decided on the other side of the
//! wire, in `nudge.ts`, so renaming a query key here is not a server change.

use serde::{Deserialize, Serialize};

// Read by the viewer off the stream, so its TypeScript comes from here — see
// `Response` for why the emitter is gated.
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// One thing that moved, said as briefly as it can be said.
///
/// A page too old to know a kind treats it as everything having moved, which is
/// the behaviour every kind used to get — so a new kind is safe to add and an
/// old page stays correct against a newer server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Nudge {
    /// A session put lines on a Conversation's Transcript.
    Transcript { conversation: i64 },

    /// A session printed, so what its terminal shows has moved — the Capture
    /// and the Screen painted from it alike.
    Screen { conversation: i64 },

    /// A commit landed on a Conversation's branch.
    Commit { conversation: i64 },

    /// A Question Set in a Conversation arrived, was answered, or was closed
    /// unanswered.
    Set { conversation: i64 },

    /// An agent took up or let go of the wait on a Question Set, so the badge
    /// saying whether anybody is listening has moved.
    ///
    /// Nothing durable changed and nothing is asked of the human: this is the
    /// one kind that is purely about a display. It exists because the verdict
    /// used to cycle with the viewer's poll, and the poll is gone (ADR-0009).
    Liveness { conversation: i64 },

    /// One Conversation moved in some other way: an Event on its Timeline, its
    /// lifecycle, the Hold on its keyboard.
    ///
    /// This is the Conversation everywhere it is drawn, its row in the sidebar
    /// included — a lifecycle that moved is a row that reads differently.
    Conversation { conversation: i64 },

    /// The membership of the list moved: a Conversation appeared or left. Which
    /// is a different thing from one of them moving, and the reason both kinds
    /// exist.
    Conversations,

    /// Something about the Repos moved, roadmaps nothing is driving included.
    Repos,

    /// The Agent Profiles moved.
    ///
    /// Nothing announces this yet: a Profile is only ever saved or deleted by a
    /// browser, which reads its own change back. It is here because the
    /// vocabulary is the taxonomy of ADR-0009 rather than a list of today's
    /// callers, and a second device watching is what it is waiting for.
    Profiles,
}
