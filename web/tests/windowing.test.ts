//! The window a checklist's card draws itself through.
//!
//! The settled examples are the ones the design was agreed on: ten entries,
//! none done, five done and nine done. Everything else here is an end of the
//! list, which is where a centred window has to stop being centred — or a list
//! worked in more than one place, which is where it stops being one slice.

import { describe, expect, it } from "vitest";

import { WINDOW, windowed } from "../src/workbench/windowing";

/// A list of `count` entries numbered from one, the first `done` of them
/// ticked — which is the shape a backlog is worked in.
function list(count: number, done: number): Entry[] {
  return Array.from({ length: count }, (_, at) => ({
    n: at + 1,
    done: at < done,
    flying: false,
  }));
}

/// One entry of these lists: what it is called, whether it is done, and whether
/// it is one of the places the work is — which is the one thing a roadmap's card
/// knows that a backlog's does not.
interface Entry {
  n: number;
  done: boolean;
  flying: boolean;
}

/// The same list with the entries numbered `flying` in flight, which is how a
/// roadmap with stages running arrives.
function running(entries: Entry[], ...flying: number[]): Entry[] {
  return entries.map((entry) => ({
    ...entry,
    flying: flying.includes(entry.n),
  }));
}

/// What the window drew, said the way the examples are: the numbers it showed,
/// and how many are hidden in front of each stretch and after the last.
function shown(entries: Entry[]) {
  const window = windowed(
    entries,
    (entry) => entry.done,
    (entry) => entry.flying,
  );

  return {
    entries: window.stretches.flatMap((stretch) =>
      stretch.entries.map((entry) => entry.n),
    ),
    hidden: window.stretches.map((stretch) => stretch.hidden),
    after: window.after,
  };
}

/// And the same, said the way the window this replaced was — one slice, with a
/// count at either end of it — which is what a list worked in one place still
/// draws.
function sliced(entries: Entry[]) {
  const window = shown(entries);

  expect(window.hidden).toHaveLength(1);

  return {
    entries: window.entries,
    before: window.hidden[0],
    after: window.after,
  };
}

describe("windowing a checklist for its card", () => {
  it("shows five entries centred on the one the work is at", () => {
    expect(sliced(list(10, 0))).toEqual({
      entries: [1, 2, 3, 4, 5],
      before: 0,
      after: 5,
    });

    expect(sliced(list(10, 5))).toEqual({
      entries: [4, 5, 6, 7, 8],
      before: 3,
      after: 2,
    });

    expect(sliced(list(10, 9))).toEqual({
      entries: [6, 7, 8, 9, 10],
      before: 5,
      after: 0,
    });
  });

  it("leaves a list that already fits alone", () => {
    for (let count = 0; count <= WINDOW; count += 1) {
      expect(sliced(list(count, 0))).toEqual({
        entries: list(count, 0).map((entry) => entry.n),
        before: 0,
        after: 0,
      });
    }

    // However far through it the work is: a short list has no end to be held
    // against, so nothing about it moves.
    expect(sliced(list(5, 3))).toEqual({
      entries: [1, 2, 3, 4, 5],
      before: 0,
      after: 0,
    });
  });

  it("shows the last five of a list with nothing left to do", () => {
    expect(sliced(list(10, 10))).toEqual({
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

    expect(sliced(entries)).toEqual({
      entries: [1, 2, 3, 4, 5],
      before: 0,
      after: 5,
    });
  });

  it("holds the window inside the list at either end", () => {
    expect(sliced(list(6, 0))).toEqual({
      entries: [1, 2, 3, 4, 5],
      before: 0,
      after: 1,
    });

    expect(sliced(list(6, 5))).toEqual({
      entries: [2, 3, 4, 5, 6],
      before: 1,
      after: 0,
    });
  });
});

/// A roadmap is worked in as many places as it has stages running, and the
/// window is over all of them: a stage in flight is what the card is read for,
/// so it is a row however many others have to give way for it.
describe("windowing a checklist worked in more than one place", () => {
  /// One place reads exactly as the box did, because one place is what a
  /// backlog and a roadmap-with-one-stage-running both have — and it is the
  /// entry in flight rather than the first one not done, which is the whole
  /// difference between the two readings.
  it("centres on the one entry in flight, wherever the boxes are", () => {
    expect(sliced(running(list(10, 5), 6))).toEqual({
      entries: [4, 5, 6, 7, 8],
      before: 3,
      after: 2,
    });

    // A stage running ahead of an unticked one: the boxes say the work is at 3
    // and the record says it is at 7, and the record is what the card follows.
    expect(sliced(running(list(10, 2), 7))).toEqual({
      entries: [5, 6, 7, 8, 9],
      before: 4,
      after: 1,
    });
  });

  /// Three of them is three rows, with the two the window has left over going
  /// either side of the three, as one entry's neighbours do.
  it("keeps every entry in flight, and fills the rest of the window around them", () => {
    expect(sliced(running(list(10, 3), 4, 5, 6))).toEqual({
      entries: [3, 4, 5, 6, 7],
      before: 2,
      after: 3,
    });
  });

  /// The window is spent on closing the gaps between them before it is spent on
  /// neighbours: an entry hidden between two in view would take the row it was
  /// hidden from and say less in it.
  it("closes the gaps between them before it reaches outwards", () => {
    expect(sliced(running(list(10, 3), 4, 6))).toEqual({
      entries: [3, 4, 5, 6, 7],
      before: 2,
      after: 3,
    });

    // Two gaps and room for the narrower one: four entries in flight leave the
    // window one row, which the gap of one takes and the gap of two cannot.
    expect(shown(running(list(20, 0), 4, 6, 9, 10))).toEqual({
      entries: [4, 5, 6, 9, 10],
      hidden: [3, 2],
      after: 10,
    });
  });

  /// And where they are too far apart to join up, the card draws them in
  /// stretches: five rows of entries as ever, with a mark saying what is out of
  /// sight in each of the gaps as well as at the ends.
  it("draws entries too far apart to join up as stretches", () => {
    expect(shown(running(list(20, 0), 3, 10, 18))).toEqual({
      entries: [2, 3, 10, 18, 19],
      hidden: [1, 6, 7],
      after: 1,
    });
  });

  /// Nothing is dropped to hold the window at five: more entries in flight than
  /// it are more rows than it, because a row is what says an entry is being
  /// worked.
  it("holds every entry in flight even where there are more than the window", () => {
    expect(shown(running(list(20, 0), 2, 4, 8, 12, 16, 19))).toEqual({
      entries: [2, 4, 8, 12, 16, 19],
      hidden: [1, 1, 3, 3, 3, 2],
      after: 1,
    });
  });

  /// The ends are held the way they are for one place: the neighbours a list
  /// has no room for on one side are taken on the other, so a roadmap running
  /// its first stages shows its first five.
  it("holds the window inside the list at either end", () => {
    expect(sliced(running(list(10, 0), 1, 2))).toEqual({
      entries: [1, 2, 3, 4, 5],
      before: 0,
      after: 5,
    });

    expect(sliced(running(list(10, 8), 9, 10))).toEqual({
      entries: [6, 7, 8, 9, 10],
      before: 5,
      after: 0,
    });
  });

  /// A list with nothing in flight is the reading this grew out of, whatever
  /// its boxes say — which is what keeps a backlog, a roadmap nobody has
  /// started and a roadmap with every stage done drawing what they drew.
  it("falls back to the first entry not done where nothing is in flight", () => {
    expect(sliced(running(list(10, 5)))).toEqual({
      entries: [4, 5, 6, 7, 8],
      before: 3,
      after: 2,
    });
  });
});
