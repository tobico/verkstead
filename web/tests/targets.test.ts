//! What a Target field holds, and what a Brief names — the viewer's own copy
//! of a reading the server has.
//!
//! Asked of the module rather than of a rendered field, for the reason the
//! role table is: what a panel drew because a `<Show>` happened to be true is
//! not what says a URL names a pull request. The server's own tests are in
//! `crates/server/src/targets.rs`, and these two are written against the same
//! cases so a drift shows up as a disagreement rather than as a field that
//! quietly draws the wrong thing.

import { describe, expect, it } from "vitest";

import { namesPullRequest, pullRequestIn } from "../src/workbench/targets";

describe("what a target field holds", () => {
  /// Whichever of the addresses GitHub hands out was copied into it.
  it("reads a pull request out of every shape of its url", () => {
    for (const field of [
      "https://github.com/tobico/verkstead/pull/41",
      "http://www.github.com/tobico/verkstead/pull/41",
      "github.com/tobico/verkstead/pull/41",
      "https://github.com/tobico/verkstead/pull/41/files",
      "https://github.com/tobico/verkstead/pull/41#issuecomment-9",
      "  https://github.com/tobico/verkstead/pull/41  ",
    ]) {
      expect(namesPullRequest(field), field).toBe(true);
    }
  });

  /// And a bare number, which names one in whichever repository the
  /// Conversation is on.
  it("reads a bare number as a pull request", () => {
    expect(namesPullRequest("#41")).toBe(true);
    expect(namesPullRequest(" #41 ")).toBe(true);
  });

  /// Everything else is a branch. The field holds the name of one thing, so
  /// prose with a number in it is a branch nobody has rather than the pull
  /// request somewhere inside it — which the server refuses by name.
  it("reads everything else as a branch", () => {
    for (const field of [
      "rate-limiting",
      "feature/rate-limiting",
      "",
      "   ",
      "fix #41 first",
      "#41 — the rate limiter",
      "41",
      "tobico/askance#41",
    ]) {
      expect(namesPullRequest(field), field).toBe(false);
    }
  });
});

describe("what a brief names", () => {
  /// A URL names a number and the repository it is in, and what goes into the
  /// field is the URL the human wrote.
  it("writes a url back as the url", () => {
    expect(
      pullRequestIn("Wrap up https://github.com/tobico/verkstead/pull/41 today."),
    ).toBe("https://github.com/tobico/verkstead/pull/41");
  });

  /// And a bare number back as a bare number: the field is the human's to
  /// read and correct, so what stands in it is their own name for the thing.
  it("writes a bare number back as a number", () => {
    expect(pullRequestIn("Please wrap #41 up.")).toBe("#41");
  });

  /// The first of either, wherever it falls — and a URL carrying its own
  /// fragment is one name rather than a URL and a `#`.
  it("takes the first name in the prose", () => {
    expect(
      pullRequestIn("#7 is the one, not https://github.com/tobico/verkstead/pull/41"),
    ).toBe("#7");

    expect(
      pullRequestIn(
        "Start from https://github.com/tobico/verkstead/pull/41#issuecomment-9 and #7 after it",
      ),
    ).toBe("https://github.com/tobico/verkstead/pull/41");
  });

  /// A heading's `#` is followed by a space rather than by digits, and
  /// GitHub's cross-repository shorthand names a repository this origin is
  /// not — so neither is read as a target at all.
  it("names nothing where the prose names nothing", () => {
    expect(pullRequestIn("# Rate limiting\n\nThe API needs a ceiling.\n")).toBeNull();
    expect(pullRequestIn("tobico/askance#41 is the one")).toBeNull();
    expect(pullRequestIn("The rate limiter needs a review.")).toBeNull();
  });
});
