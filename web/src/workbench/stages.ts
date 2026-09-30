//! The words a roadmap stage's state is said in.
//!
//! The wire carries a state as a name — the server's own reading of where the
//! stage is, off Verkstead's record of the stage Conversations and the boxes in
//! `ROADMAP.md` together — and what the human reads is the page's to choose.
//! The viewer picks the words and works out none of the states: a card that
//! decided for itself whether a stage was under way would be a second opinion
//! about something the record already knows.
//!
//! In one place because the same state is written in three: the row on the
//! pinned card, the heading of the stage's section in the details pane, and
//! that pane's table-of-contents line. A state worded two ways is two states
//! to the person reading them.

import type { StageState } from "../api/types";

/// What each state is called on the page.
///
/// Lower case, because these are words in a row rather than names of anything —
/// the same way the card has always said *done* and *to do*.
export const STAGE_STATE: Record<StageState, string> = {
  Done: "done",
  InProgress: "in progress",
  Halted: "halted",
  ToDo: "to do",
};

/// Whether a stage's work is over, which is the one thing the page still asks
/// of a state rather than only saying: the row is struck through, its box is
/// ticked, and the card counts it against the roadmap's length.
export function settled(state: StageState): boolean {
  return state === "Done";
}
