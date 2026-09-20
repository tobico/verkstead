//! The words a Conversation's conditions are said in.
//!
//! Worth a test of its own for the reason the module is worth having: the same
//! condition is drawn twice — on the card the human opens and on the row they
//! find it by — and what is being pinned here is the sentence itself, once, for
//! both of them. Where each lands is the pages' own tests' to say.

import { describe, expect, it } from "vitest";

import { parked } from "../src/workbench/conditions";

/// A session that has been sitting there for `idle_seconds`, spoken to
/// `spoken_to` times.
function sat(idle_seconds: number, spoken_to: number): string {
  return parked({ idle_seconds, spoken_to });
}

describe("what a session sitting there is said in", () => {
  /// The sentence the human reads on a parked agent: how long, and how far
  /// Verkstead has got with it.
  it("says the span and the count", () => {
    expect(sat(260, 1)).toBe("idle 4 min, spoken to once");
    expect(sat(260, 2)).toBe("idle 4 min, spoken to twice");
  });

  /// The span alone for a count this viewer should never see. The condition is
  /// drawn on the Rescue having spoken, so a nought is a server that does not
  /// agree with this one about what the condition is — and *spoken to no times*
  /// is a sentence about nothing.
  it("says the span alone for a count it should never see", () => {
    expect(sat(260, 0)).toBe("idle 4 min");
  });

  /// The largest unit that leaves a whole number in front of it, rounded down
  /// the way an age is: half a minute more is nothing the reader decides
  /// anything on, and a condition counting seconds would redraw every second
  /// and say no more for it.
  it("counts in the unit a reader would use", () => {
    expect(sat(4, 0)).toBe("idle 4 s");
    expect(sat(59, 0)).toBe("idle 59 s");
    expect(sat(60, 0)).toBe("idle 1 min");
    expect(sat(3599, 0)).toBe("idle 59 min");
    expect(sat(7200, 0)).toBe("idle 2 h");
  });

  /// Twice is the last of them — the third silence is the stop — so a count
  /// past it is one this viewer is older than. Said in figures rather than left
  /// unsaid: a number nobody has words for is still worth reading.
  it("says a count it has no word for in figures", () => {
    expect(sat(260, 3)).toBe("idle 4 min, spoken to 3 times");
  });
});
