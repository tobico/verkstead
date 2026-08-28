//! The window a checklist's card draws itself through.
//!
//! The settled examples are the ones the design was agreed on: ten entries,
//! none done, five done and nine done. Everything else here is an end of the
//! list, which is where a centred window has to stop being centred.

import { describe, expect, it } from "vitest";

import { WINDOW, windowed } from "../src/workbench/windowing";

/// A list of `count` entries numbered from one, the first `done` of them
/// ticked — which is the shape a backlog is worked in.
function list(count: number, done: number): { n: number; done: boolean }[] {
  return Array.from({ length: count }, (_, at) => ({
    n: at + 1,
    done: at < done,
  }));
}

/// What the window drew, said the way the examples are.
function shown(entries: { n: number; done: boolean }[]) {
  const window = windowed(entries, (entry) => entry.done);
  return {
    entries: window.entries.map((entry) => entry.n),
    before: window.before,
    after: window.after,
  };
}

describe("windowing a checklist for its card", () => {
  it("shows five entries centred on the one the work is at", () => {
    expect(shown(list(10, 0))).toEqual({
      entries: [1, 2, 3, 4, 5],
      before: 0,
      after: 5,
    });

    expect(shown(list(10, 5))).toEqual({
      entries: [4, 5, 6, 7, 8],
      before: 3,
      after: 2,
    });

    expect(shown(list(10, 9))).toEqual({
      entries: [6, 7, 8, 9, 10],
      before: 5,
      after: 0,
    });
  });

  it("leaves a list that already fits alone", () => {
    for (let count = 0; count <= WINDOW; count += 1) {
      expect(shown(list(count, 0))).toEqual({
        entries: list(count, 0).map((entry) => entry.n),
        before: 0,
        after: 0,
      });
    }

    // However far through it the work is: a short list has no end to be held
    // against, so nothing about it moves.
    expect(shown(list(5, 3))).toEqual({
      entries: [1, 2, 3, 4, 5],
      before: 0,
      after: 0,
    });
  });

  it("shows the last five of a list with nothing left to do", () => {
    expect(shown(list(10, 10))).toEqual({
      entries: [6, 7, 8, 9, 10],
      before: 5,
      after: 0,
    });
  });

  /// The entry being worked is not always the first undone one — a list can be
  /// ticked out of order — and what the window follows is the first box that is
  /// still empty, because that is the work that is left.
  it("centres on the first entry that is not done, however the rest are ticked", () => {
    const entries = list(10, 10).map((entry) => ({
      ...entry,
      done: entry.n !== 3 && entry.n !== 8,
    }));

    expect(shown(entries)).toEqual({
      entries: [1, 2, 3, 4, 5],
      before: 0,
      after: 5,
    });
  });

  it("holds the window inside the list at either end", () => {
    expect(shown(list(6, 0))).toEqual({
      entries: [1, 2, 3, 4, 5],
      before: 0,
      after: 1,
    });

    expect(shown(list(6, 5))).toEqual({
      entries: [2, 3, 4, 5, 6],
      before: 1,
      after: 0,
    });
  });
});
