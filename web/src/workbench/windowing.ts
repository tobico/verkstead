//! How much of a checklist its card shows: five entries around the one the work
//! is at, and a mark where the rest of them are.
//!
//! A backlog and a roadmap are both pinned above the record, where they stay
//! for the whole of a Conversation, and a card that grew with the list would
//! push everything the human came to read off the screen. What is worth seeing
//! there is where the work has got to — the entry being worked, what it came
//! out of and what it goes into — so the card keeps a window of five around it
//! and the details pane keeps the whole list.
//!
//! Shared by the task list and the stage list because they are the same card
//! one level apart, and a window that read differently on the two would be two
//! ideas of where the work is.

/// How many entries a card draws. Five is what fits above the record on a
/// phone without the record itself going under the fold.
export const WINDOW = 5;

/// A list cut down to what its card shows: the entries themselves, and how many
/// are out of sight above and below them.
export interface Window<T> {
  /// The entries to draw, in the list's own order.
  entries: T[];

  /// How many entries are hidden before the first of them, and after the last.
  /// Either being more than none is what puts an ellipsis row at that end.
  before: number;
  after: number;
}

/// The window of `entries` centred on the first one that is not done, held
/// inside the list's ends.
///
/// Centred rather than led by, because an entry is read against its
/// neighbours: what the work just finished says as much about where it is as
/// what it is about to start. The clamp is what makes the ends read the way a
/// list does — the start of a backlog shows its first five and the end of one
/// shows its last five, rather than a half-empty window hanging off either end.
///
/// A list with every entry done is at its end: stage lists outlive their
/// completion, and the last five are what a finished list is looked at for.
export function windowed<T>(
  entries: T[],
  done: (entry: T) => boolean,
): Window<T> {
  if (entries.length <= WINDOW) {
    return { entries, before: 0, after: 0 };
  }

  const next = entries.findIndex((entry) => !done(entry));
  const at = next === -1 ? entries.length : next;

  const before = Math.min(
    Math.max(at - Math.floor(WINDOW / 2), 0),
    entries.length - WINDOW,
  );

  return {
    entries: entries.slice(before, before + WINDOW),
    before,
    after: entries.length - before - WINDOW,
  };
}
