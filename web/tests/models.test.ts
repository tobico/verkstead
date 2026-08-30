//! The models the viewer knows the names of, and what it does about one it does
//! not.
//!
//! The fallback is the whole reason there is a test here. A list of the models
//! there are goes stale the week another one ships, so the interesting case is
//! not `claude-opus-5` reading "Opus 5" — it is what a build a year old says
//! about an id nobody had written when it shipped.

import { describe, expect, it } from "vitest";

import { KNOWN_MODELS, known, prettify } from "../src/models";

describe("the known models", () => {
  it("gives every one of them a pretty name and an id", () => {
    expect(KNOWN_MODELS.length).toBeGreaterThan(0);

    for (const model of KNOWN_MODELS) {
      expect(model.id).toMatch(/^claude-/);
      expect(model.name).not.toBe("");
      expect(model.name).not.toBe(model.id);
    }
  });

  /// Two entries under one id would make [`prettify`] answer whichever the map
  /// was built with last, which is a way of being quietly wrong.
  it("lists each id once", () => {
    const ids = KNOWN_MODELS.map((model) => model.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("reads a known id as its name", () => {
    expect(prettify("claude-opus-5")).toBe("Opus 5");
    expect(prettify("claude-fable-5")).toBe("Fable 5");
    expect(known("claude-opus-5")).toBe(true);
  });

  /// The list going stale is the ordinary case rather than the error one: the id
  /// comes back unchanged, so a model this build has never heard of still reads
  /// as something on the button that shows it.
  it("hands back an id it does not know, unchanged", () => {
    expect(prettify("claude-opus-7")).toBe("claude-opus-7");
    expect(prettify("")).toBe("");
    expect(known("claude-opus-7")).toBe(false);
  });
});
