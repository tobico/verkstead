//! Nothing in the viewer is set in capitals.
//!
//! The badges used to be: `LIVE`, `DEFERRED`, `CLOSED`, the header's `BLOCKED ON
//! YOU`, the per-file pills on a Diff and the one on a Set this build cannot
//! read. Six stylesheets asked for it and every one of them wrote the words in
//! sentence case underneath, so the capitals were a treatment rather than
//! anything anybody typed — and a page of them reads as a page being shouted at.
//!
//! Guarded across every stylesheet rather than the six, because the rule is
//! about the viewer rather than about those badges: the next badge is written in
//! the case its words were typed in too.

import { describe, expect, it } from "vitest";

/// Every stylesheet under `src/`, as its source — the modules and the three
/// global sheets alike.
const SHEETS = import.meta.glob("../src/**/*.css", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

describe("the viewer's type", () => {
  /// The glob is the whole of what this test is worth, so a glob that found
  /// nothing would pass while asserting nothing at all.
  it("reads every stylesheet in the viewer", () => {
    expect(Object.keys(SHEETS).length).toBeGreaterThan(10);
  });

  it("asks for capitals nowhere", () => {
    const shouting = Object.entries(SHEETS).filter(([, source]) =>
      /text-transform:\s*(uppercase|capitalize)/.test(source),
    );

    expect(shouting.map(([path]) => path)).toEqual([]);
  });
});
