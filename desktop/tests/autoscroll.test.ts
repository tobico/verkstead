//! What the app tells Chromium so that a held middle button scrolls.
//!
//! There is no platform in it and so no arm to pick, which is the whole of what
//! this pins: two strings that are somebody else's spelling rather than this
//! project's, and the pair they are appended as. An unrecognised Blink feature
//! is ignored in silence rather than refused, so a name tidied by hand is an app
//! that starts, says in the log that autoscroll is on, and does not autoscroll —
//! which is why the strings are asserted rather than left to read well.
//!
//! **Against the strings themselves and never against the module's own
//! constants.** `AUTOSCROLLING` is built out of the two below it, so asserting
//! it against `{ name: FEATURES, value: AUTOSCROLL }` is the module agreeing
//! with itself: it passes whatever those two have been tidied to, which is the
//! one failure there is to catch here.

import { describe, expect, it } from "vitest";

import { AUTOSCROLL, AUTOSCROLLING, FEATURES } from "../src/autoscroll.js";

describe("the switch the app appends", () => {
  /// Both strings and the pairing in one: which of them is the switch's name and
  /// which is its value is the other thing a swap would get wrong.
  it("asks Blink for middle-click autoscroll", () => {
    expect(AUTOSCROLLING).toEqual({
      name: "enable-blink-features",
      value: "MiddleClickAutoscroll",
    });
  });

  it("is the switch Chromium reads runtime features off", () => {
    expect(FEATURES).toBe("enable-blink-features");
  });

  it("naming the feature as Blink spells it", () => {
    expect(AUTOSCROLL).toBe("MiddleClickAutoscroll");
  });
});
