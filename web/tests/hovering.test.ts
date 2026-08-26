//! Every hover state in the viewer is behind `@media (hover: hover)`, and
//! everything that can be pressed has one.
//!
//! A hover rule a touch device can reach is a rule it paints on whatever was
//! last tapped and leaves there: the finger has gone and nothing tells the page
//! so. Firefox and Safari on a phone both do it, and the fix is the query rather
//! than a script — `hover: hover` is exactly "there is a pointer that can rest
//! on things".
//!
//! So the sweep that gave the whole viewer hover states is guarded from the
//! source rather than from a rendering: a stylesheet is where the mistake is
//! made, and it is made by writing `:hover` one line above the block instead of
//! one line below it.

import { describe, expect, it } from "vitest";

/// Every stylesheet under `src/`, as its source — the modules and the three
/// global sheets alike.
const SHEETS = import.meta.glob("../src/**/*.css", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/// Comments out. They are prose about the rules, and the prose says "hover"
/// rather more often than the rules do.
function bare(source: string): string {
  return source.replace(/\/\*[\s\S]*?\*\//g, "");
}

/// A media query that asks for a pointer that can rest on things.
const HOVER_QUERY = /@media[^{]*\(\s*hover\s*:\s*hover\s*\)/;

/// Every block whose selector carries `:hover`, with the preludes it is nested
/// inside — the outermost first. Walking the braces rather than parsing: what
/// this has to know is which `@media` a rule is under, and that is the stack.
function hovers(source: string): { selector: string; within: string[] }[] {
  const found: { selector: string; within: string[] }[] = [];
  const stack: string[] = [];
  let prelude = "";

  for (const char of bare(source)) {
    if (char === "{") {
      const opened = prelude.trim();
      if (opened.includes(":hover")) {
        found.push({ selector: opened, within: [...stack] });
      }
      stack.push(opened);
      prelude = "";
    } else if (char === "}") {
      stack.pop();
      prelude = "";
    } else if (char === ";") {
      prelude = "";
    } else {
      prelude += char;
    }
  }

  return found;
}

describe("the viewer's hover states", () => {
  /// The glob is the whole of what this test is worth, so a glob that found
  /// nothing would pass while asserting nothing at all.
  it("reads every stylesheet in the viewer", () => {
    expect(Object.keys(SHEETS).length).toBeGreaterThan(10);
  });

  it("writes none of them outside the hover query", () => {
    const loose = Object.entries(SHEETS).flatMap(([path, source]) =>
      hovers(source)
        .filter(({ within }) => !within.some((at) => HOVER_QUERY.test(at)))
        .map(({ selector }) => `${path}: ${selector}`),
    );

    expect(loose).toEqual([]);
  });

  /// Coarse on purpose: a sheet is the unit a hover state is written in, and a
  /// sheet that says something can be pressed and never says what that looks
  /// like under the pointer is the shape the sweep was for.
  it("gives one to everything that says it can be pressed", () => {
    const quiet = Object.entries(SHEETS)
      .filter(([, source]) => /cursor:\s*pointer/.test(bare(source)))
      .filter(([, source]) => !HOVER_QUERY.test(bare(source)))
      .map(([path]) => path);

    expect(quiet).toEqual([]);
  });
});
