//! What the app tells Chromium so that a held middle button scrolls.
//!
//! There is no platform in it and so no arm to pick, which is the whole of what
//! this pins: two strings that are somebody else's spelling rather than this
//! project's, and the pair they are appended as. An unrecognised Blink feature
//! is ignored in silence rather than refused, so a name tidied by hand is an app
//! that starts, says in the log that autoscroll is on, and does not autoscroll —
//! which is why the strings are asserted rather than left to read well.

import { describe, expect, it } from "vitest";

import { AUTOSCROLL, AUTOSCROLLING, FEATURES } from "../src/autoscroll.js";

describe("the switch the app appends", () => {
  it("asks Blink for middle-click autoscroll", () => {
    expect(AUTOSCROLLING).toEqual({ name: FEATURES, value: AUTOSCROLL });
  });

  it("is the switch Chromium reads runtime features off", () => {
    expect(FEATURES).toBe("enable-blink-features");
  });

  it("naming the feature as Blink spells it", () => {
    expect(AUTOSCROLL).toBe("MiddleClickAutoscroll");
  });

  /// One `appendSwitch` owns the whole value, and several features are a
  /// comma-joined list — so a value with a comma in it would be this module
  /// having quietly become the place two features are asked for.
  it("asks for the one feature and no list", () => {
    expect(AUTOSCROLLING.value).not.toContain(",");
  });
});
