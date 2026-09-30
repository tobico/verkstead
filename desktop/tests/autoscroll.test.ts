//! Which platforms are told to autoscroll, and what they are told.
//!
//! Three arms and this machine runs one of them, which is the reason the module
//! takes the platform as a value: a Mac and a Windows are the two that are told
//! nothing, for opposite reasons, and neither reason can be read off the
//! process here.

import { describe, expect, it } from "vitest";

import { AUTOSCROLL, autoscrolling, FEATURES } from "../src/autoscroll.js";

describe("on Linux", () => {
  it("asks Blink for middle-click autoscroll", () => {
    expect(autoscrolling("linux")).toEqual({ name: FEATURES, value: AUTOSCROLL });
  });

  /// The other Unixes go the same way, as they do for the Data Directory: the
  /// arm is *not a Mac and not Windows* rather than a list of one.
  it("and so do the other Unixes", () => {
    expect(autoscrolling("freebsd")).toEqual({ name: FEATURES, value: AUTOSCROLL });
  });
});

/// Chromium enables the feature there itself, so the switch would say something
/// already true.
describe("on Windows", () => {
  it("asks for nothing, the feature being on already", () => {
    expect(autoscrolling("win32")).toBeUndefined();
  });
});

/// And a Mac is not a platform that autoscrolls at all.
describe("on a Mac", () => {
  it("asks for nothing", () => {
    expect(autoscrolling("darwin")).toBeUndefined();
  });
});

/// The two strings, pinned rather than left to read well: the switch is
/// Chromium's own spelling and the feature is Blink's, and an unrecognised
/// feature name is ignored in silence rather than refused — so a tidied one is
/// an app that starts, logs that autoscroll is on, and does not autoscroll.
describe("what is appended", () => {
  it("is the switch Chromium reads runtime features off", () => {
    expect(FEATURES).toBe("enable-blink-features");
  });

  it("naming the feature as Blink spells it", () => {
    expect(AUTOSCROLL).toBe("MiddleClickAutoscroll");
  });
});
