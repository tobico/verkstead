//! The models the viewer knows the names of, and what it does about one it does
//! not.
//!
//! The fallback is the whole reason there is a test here. A list of the models
//! there are goes stale the week another one ships, so the interesting case is
//! not `claude-opus-5` reading "Opus 5" — it is what a build a year old says
//! about an id nobody had written when it shipped.

import { describe, expect, it } from "vitest";

import { AGENT_NAME } from "../src/agents";
import { KNOWN_MODELS, known, prettify } from "../src/models";

describe("the known models", () => {
  /// A backend apiece as well as a name, because the profile form offers one
  /// backend's models at a time: an entry naming a backend nobody has a name for
  /// is one nothing would offer.
  it("gives every one of them a pretty name, an id and a backend", () => {
    expect(KNOWN_MODELS.length).toBeGreaterThan(0);

    for (const model of KNOWN_MODELS) {
      expect(model.id).not.toBe("");
      expect(model.name).not.toBe("");
      expect(model.name).not.toBe(model.id);
      expect(AGENT_NAME[model.agent]).toBeTruthy();
    }
  });

  /// Every backend that can launch has at least one model written down. Not a
  /// rule about the world — an account can be given an id by hand — but the
  /// picks are the ordinary way in, and a backend with none would be a form that
  /// looked broken until somebody guessed at a spelling.
  it("knows a model for every backend", () => {
    for (const agent of Object.keys(AGENT_NAME)) {
      expect(
        KNOWN_MODELS.some((model) => model.agent === agent),
        `nothing is written down for ${agent}`,
      ).toBe(true);
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
