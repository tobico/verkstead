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

/// The sheets that do, and why: the setup wizard's small labels — *Setup*
/// under the brand, *Step 1 of 3* over the title, and the first step's group
/// labels, *Required*, *An agent* and *Optional* — are set in tracked
/// capitals in the approved *B · Rail* design (`docs/design/setup-wizard/`),
/// and the backlog that builds it asks for them. Labels rather than badges, at
/// 11.5px: the words are still typed in sentence case, and only these sheets
/// may set them in capitals.
const LABELLED = [
  "../src/setup/SetupPage.module.css",
  "../src/setup/Dependencies.module.css",
  "../src/setup/Accounts.module.css",
];

describe("the viewer's type", () => {
  /// The glob is the whole of what this test is worth, so a glob that found
  /// nothing would pass while asserting nothing at all.
  it("reads every stylesheet in the viewer", () => {
    expect(Object.keys(SHEETS).length).toBeGreaterThan(10);
  });

  it("asks for capitals nowhere", () => {
    const shouting = Object.entries(SHEETS).filter(
      ([path, source]) =>
        !LABELLED.includes(path) &&
        /text-transform:\s*(uppercase|capitalize)/.test(source),
    );

    expect(shouting.map(([path]) => path)).toEqual([]);
  });
});
