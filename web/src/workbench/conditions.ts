//! The words a Conversation's derived conditions are said in.
//!
//! A condition is something true *of* a state rather than a state of its own —
//! the ladder is untouched and nothing new is stored — so it is drawn beside the
//! lifecycle word rather than in place of one on the wire. What it is called is
//! this module's, in one place, because the same condition is said twice: once
//! on the card the human opens and once in the sidebar row they find it by, and
//! a condition worded two ways is two conditions to the person reading them.

import type { Parked } from "../api/types";

/// A wrap-up that has got down to its checks: the review answered, nothing said
/// on the pull request left unaddressed, and nothing running in the Worktree.
///
/// *Checks* rather than *CI*, which is the codebase's word throughout: a
/// GitHub check may be a required review or a deploy gate as easily as a test
/// suite, and the human is waiting on all of them alike.
export const WAITING_ON_CHECKS = "Waiting on checks";

/// A running session the Rescue is watching sit there: how long it has been
/// idle, and how many times it has been spoken to — *idle 4 min, spoken to
/// once*.
///
/// The condition a parked agent wears and a working one does not, which is the
/// whole of what it is for: a session with its turn over, nothing open on the
/// Conversation and nothing to show for itself reads on the card exactly like
/// one hard at work, and somebody watched one for ten minutes before concluding
/// nothing had been captured.
///
/// Both halves, because either alone leaves the reader guessing: the span says
/// how long this has been going on, and the count says that Verkstead has
/// noticed and is doing something about it. The count is left off before the
/// first line is typed — *spoken to no times* is a sentence about nothing.
///
/// Minutes past the first, because the number is read rather than timed: a
/// condition counting seconds redraws every second and says no more for it. The
/// seconds below the minute are still said, for the machine whose grace is
/// shorter than one.
export function parked(condition: Parked): string {
  const idle = `idle ${span(condition.idle_seconds)}`;

  // Nothing said about the count until there is one. The rescue arms on the
  // same grace this is drawn past, so every session wears the span alone for a
  // poll or two before it is spoken to at all.
  if (condition.spoken_to === 0) {
    return idle;
  }

  return `${idle}, ${SPOKEN_TO[condition.spoken_to] ?? `spoken to ${condition.spoken_to} times`}`;
}

/// How many times it has been told, in words rather than in a number: a session
/// is spoken to once and then twice, and *spoken to once* is what somebody
/// would say about it.
///
/// Twice is the last of them — the third silence is the stop — so anything past
/// it is a count this viewer is older than, and those are said in figures
/// rather than left unsaid.
const SPOKEN_TO: Record<number, string> = {
  1: "spoken to once",
  2: "spoken to twice",
};

/// A span of idle, in the largest unit that leaves a whole number in front of
/// it.
///
/// Rounded down, as an age is: a session that has been sitting there for four
/// and a half minutes has been sitting there for four minutes, and the half is
/// nothing the reader is deciding anything on.
function span(seconds: number): string {
  if (seconds < 60) {
    return `${seconds} s`;
  }

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) {
    return `${minutes} min`;
  }

  return `${Math.floor(minutes / 60)} h`;
}
