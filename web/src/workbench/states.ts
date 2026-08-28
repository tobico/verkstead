//! The words a Conversation's lifecycle states are said in.
//!
//! The wire carries a state as a name rather than a sentence — the record's own
//! word for where the work has got to — and what the human reads is the page's
//! to choose. For five of the seven the two are the same word, and this exists
//! for the ones where they are not: *Follow-up* is a hyphenated word everywhere
//! in this product, and `FollowUp` is only how a variant is spelled.
//!
//! In one place because the same state is written in three: the move on a
//! Timeline, the steer above it, and the sidebar row read aloud. A state worded
//! two ways is two states to the person reading them.

import type { Lifecycle } from "../api/types";

/// What each state is called on the page.
export const STATE: Record<Lifecycle, string> = {
  Draft: "Draft",
  Grilling: "Grilling",
  Implementing: "Implementing",
  Wrapping: "Wrapping",
  FollowUp: "Follow-up",
  Done: "Done",
  Closed: "Closed",
};

/// The states the work has ended in: the ladder's last rung, and the way off
/// the ladder. Neither has anything running or anything to come, which is what
/// makes them worth saying in a header where the states on the way say nothing.
export const ENDED: ReadonlySet<Lifecycle> = new Set<Lifecycle>([
  "Done",
  "Closed",
]);
