//! The words a Conversation's lifecycle states are said in.
//!
//! The wire carries a state as a name rather than a sentence — the record's own
//! word for where the work has got to — and what the human reads is the page's
//! to choose. For seven of the eight the two are the same word, and this exists
//! for the one where they are not: *Follow-up* is a hyphenated word everywhere
//! in this product, and `FollowUp` is only how a variant is spelled.
//!
//! In one place because the same state is written in three: the move on a
//! Timeline, the steer above it, and the sidebar row read aloud. A state worded
//! two ways is two states to the person reading them.

import type { Lifecycle, ProvidedProcess } from "../api/types";
import { declaredWord } from "./provided";

/// What each state is called on the page.
export const STATE: Record<Lifecycle, string> = {
  Draft: "Draft",
  Grilling: "Grilling",
  Implementing: "Implementing",
  Wrapping: "Wrapping",
  FollowUp: "Follow-up",
  Investigating: "Investigating",
  // One a plugin's Process declares, whose word is its manifest's — see
  // `declaredWord` in `provided.ts`. This is only ever read where the state's
  // id has gone missing.
  Declared: "In progress",
  Done: "Done",
  Closed: "Closed",
};

/// The states the work has ended in: the ladder's last rung, and the way off
/// the ladder. Neither has anything running or anything to come, which is what
/// makes them the two the status button says on their own — no status word
/// beside the state, because the word for where a finished Conversation got to
/// *is* the state, and saying it twice with a colour between would be the
/// button repeating itself.
export const ENDED: ReadonlySet<Lifecycle> = new Set<Lifecycle>([
  "Done",
  "Closed",
]);

/// What a state is called on the page, a declared one included: the word its
/// Process's manifest gives the state `declared`, where `state` is one a
/// plugin's Process declares — see `provided.ts`.
export function stateWord(
  state: Lifecycle,
  provided: ProvidedProcess | null,
  declared: string | null | undefined,
): string {
  return state === "Declared" && declared
    ? declaredWord(provided, declared)
    : STATE[state];
}
