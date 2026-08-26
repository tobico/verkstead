//! Whether an agent is still on the other end of a Question Set.

use serde::{Deserialize, Serialize};

// Drawn by the viewer, so its TypeScript comes from here — see `Response`.
#[cfg(feature = "typescript")]
use ts_rs::TS;

/// What a Set still waiting on the human says about itself: whether an agent is
/// currently waiting on it, whether nothing is holding a wait any more, or
/// whether nothing ever was.
///
/// Display state only (ADR-0001). A disconnected Set is still answerable and is
/// never withdrawn on its own — the CLI reconnects through transient drops, and
/// only a human may lock a Set whose agent is really gone.
///
/// It is a verdict rather than a timestamp because the server has the clock and
/// the registry of held waits; the browser only draws what it is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
#[serde(rename_all = "snake_case")]
pub enum Liveness {
    /// A wait is held on the Set, or one was held recently enough that its
    /// agent is taken to be between polls.
    Waiting,

    /// Nothing has held a wait on the Set for long enough that its agent is
    /// taken to be gone.
    Disconnected,

    /// It was a Deferred Ask: no wait was ever held on it, and none ever will
    /// be. The session that asked went on working and its Answers reach a later
    /// one.
    ///
    /// A verdict of its own rather than the absence of one, because the question
    /// the badge answers — is anyone still on the other end? — has three
    /// answers rather than two, and *disconnected* would be this build reporting
    /// an agent that had gone where none was ever waiting. It is answerable and
    /// lockable exactly as the other two are: what differs is who is waiting,
    /// which is nobody.
    Deferred,
}
