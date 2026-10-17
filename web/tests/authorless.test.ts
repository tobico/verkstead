//! What a start and an adoption say over a repo with no author of its own: one
//! `config.yaml` no longer holds, or none picked among several (ADR-0025).
//!
//! The same sentence at both doors, naming the repo and sending the human to
//! where another is picked.

import { describe, expect, it } from "vitest";

import { adoptRefusal } from "../src/workbench/Adoption";
import { grillRefusal } from "../src/workbench/Timeline";

describe("a repo with no author of its own", () => {
  it("is named, and sent to the repos pane, when its author is gone", () => {
    const said = grillRefusal({ RepoAuthorGone: { repo: "verkstead" } });

    expect(said).toBe(
      "The verkstead repo's author isn't configured any more, so Verkstead can't commit on the branch. Pick another under Repos in Settings first.",
    );
    expect(adoptRefusal({ RepoAuthorGone: { repo: "verkstead" } })).toBe(said);
  });

  it("is told to pick one where none is picked", () => {
    const said = grillRefusal({ RepoAuthorUnpicked: { repo: "verkstead" } });

    expect(said).toBe(
      "The verkstead repo has no author picked, so Verkstead can't commit on the branch. Pick one under Repos in Settings first.",
    );
    expect(adoptRefusal({ RepoAuthorUnpicked: { repo: "verkstead" } })).toBe(
      said,
    );
  });
});
