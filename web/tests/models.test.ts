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

  /// The short names a harness answers to, which a profile filled in by hand
  /// holds: read back as the model they are short for, so a pairing picked off
  /// one does not read as the raw word.
  it("reads a harness's own short names", () => {
    expect(prettify("fable")).toBe("Fable 5");
    expect(prettify("opus")).toBe("Opus 5");
    expect(prettify("sonnet")).toBe("Sonnet 5");
    expect(prettify("haiku")).toBe("Haiku 4.5");
    expect(known("fable")).toBe(true);
  });

  /// And never offered as picks, which is the whole of why they are not
  /// entries: a form filled in from this list says which model it meant.
  it("offers none of them as a pick", () => {
    const ids = KNOWN_MODELS.map((model) => model.id);

    for (const alias of ["fable", "opus", "sonnet", "haiku"]) {
      expect(ids).not.toContain(alias);
    }
  });

  /// The long context, which is a spelling of a model rather than another
  /// model: two of them are worth picking off a list, and the rule behind them
  /// reads the `[1m]` of anything the build can already read.
  it("reads a long-context variant off whatever its base reads as", () => {
    expect(prettify("opus[1m]")).toBe("Opus 5 (1M context)");
    expect(prettify("sonnet[1m]")).toBe("Sonnet 5 (1M context)");
    expect(prettify("claude-opus-5[1m]")).toBe("Opus 5 (1M context)");
    expect(prettify("fable[1m]")).toBe("Fable 5 (1M context)");
    expect(known("claude-opus-5[1m]")).toBe(true);

    // And nothing invented over an id nothing else could read either.
    expect(prettify("claude-opus-7[1m]")).toBe("claude-opus-7[1m]");
    expect(known("claude-opus-7[1m]")).toBe(false);
  });

  /// And spelled out in full where they are offered, like every other entry: a
  /// pick says exactly which model it meant, and `opus[1m]` is the harness's
  /// short name for whichever one it calls Opus this month.
  it("offers the two that are worth a row, in full", () => {
    const ids = KNOWN_MODELS.map((model) => model.id);

    expect(ids).toContain("claude-opus-5[1m]");
    expect(ids).toContain("claude-sonnet-5[1m]");
    expect(ids).not.toContain("opus[1m]");
    expect(ids).not.toContain("sonnet[1m]");
  });
});
