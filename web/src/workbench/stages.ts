//! The words a roadmap stage's state is said in.
//!
//! The wire carries a state as a name — the server's own reading of where the
//! stage is, off Verkstead's record of the stage Conversations, the boxes in
//! `ROADMAP.md`, what each line declares it stands on, which stages the server is
//! holding before their finish and how many places it has left, all together —
//! and what the human reads is the page's to choose. The viewer picks the words and works out none of the states:
//! a card that decided for itself whether a stage was under way, or which of its
//! neighbours it was behind, would be a second opinion about something the record
//! already knows.
//!
//! In one place because the same state is written in three: the row on the
//! pinned card, the heading of the stage's section in the details pane, and
//! that pane's table-of-contents line. A state worded two ways is two states
//! to the person reading them.

import type { StageState } from "../api/types";

/// What a state is called on the page.
///
/// Lower case, because these are words in a row rather than names of anything —
/// the same way the card has always said *done* and *to do*.
///
/// One of them names stages rather than only saying a word: a stage that has not
/// started and whose line declared what it stands on says which of those it is
/// still behind, which the server sends along with the state and the page joins
/// up with commas the way it says what a stage stands on.
///
/// And one word covers two things: a stage waiting for a place is waiting on one
/// of the two limits there are — its roadmap's and the server's — and which of
/// them is said on the Timeline of the Conversation that held it rather than
/// here, a row having no room for it and the reader's next move being the only
/// thing the difference changes.
export function stageState(state: StageState): string {
  switch (state.state) {
    case "Done":
      return "done";
    case "InProgress":
      return "in progress";
    case "WaitingToJoin":
      return "waiting to join";
    case "Halted":
      return "halted";
    case "WaitingOn":
      return `waiting on ${state.stages.join(", ")}`;
    case "WaitingForAPlace":
      return "waiting for a place";
    case "ToDo":
      return "to do";
  }
}

/// Whether a stage's work is over, which is the one thing the page still asks
/// of a state rather than only saying: the row is struck through, its box is
/// ticked, and the card counts it against the roadmap's length.
export function settled(state: StageState): boolean {
  return state.state === "Done";
}
