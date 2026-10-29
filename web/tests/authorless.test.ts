//! What a start, an adoption and a take-up say over a repo with no author of its
//! own to work as: one the one list no longer holds, none picked among
//! several, or one whose token hasn't reached this device (ADR-0025).
//!
//! The same sentence at every door, naming the repo and sending the human to
//! where another is picked.

import { describe, expect, it } from "vitest";

import { adoptRefusal } from "../src/workbench/Adoption";
import { takeUpRefusal } from "../src/workbench/TakeUp";
import { grillRefusal } from "../src/workbench/Timeline";

describe("a repo with no author of its own", () => {
  it("is named, and sent to the repos pane, when its author is gone", () => {
    const said = grillRefusal({ RepoAuthorGone: { repo: "verkstead" } });

    expect(said).toBe(
      "The verkstead repo's author isn't configured any more, so Verkstead can't commit on the branch. Pick another under Repos in Settings first.",
    );
    expect(adoptRefusal({ RepoAuthorGone: { repo: "verkstead" } })).toBe(said);
    expect(takeUpRefusal({ RepoAuthorGone: { repo: "verkstead" } })).toBe(said);
  });

  it("is told to pick one where none is picked", () => {
    const said = grillRefusal({ RepoAuthorUnpicked: { repo: "verkstead" } });

    expect(said).toBe(
      "The verkstead repo has no author picked, so Verkstead can't commit on the branch. Pick one under Repos in Settings first.",
    );
    expect(adoptRefusal({ RepoAuthorUnpicked: { repo: "verkstead" } })).toBe(
      said,
    );
    expect(takeUpRefusal({ RepoAuthorUnpicked: { repo: "verkstead" } })).toBe(
      said,
    );
  });

  it("says an author's token is still to come from another device", () => {
    const awaited = { repo: "verkstead" };
    const said = grillRefusal({ RepoAuthorTokenAwaited: awaited });

    expect(said).toBe(
      "The verkstead repo's author has a token on another device, and it hasn't reached this one yet. Try again once a device holding it is reachable.",
    );
    expect(adoptRefusal({ RepoAuthorTokenAwaited: awaited })).toBe(said);
    expect(takeUpRefusal({ RepoAuthorTokenAwaited: awaited })).toBe(said);
  });

  it("is told to set a git author at a take-up, as at a start", () => {
    expect(takeUpRefusal("NoGitAuthor")).toBe(grillRefusal("NoGitAuthor"));
  });
});
