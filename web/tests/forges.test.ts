//! A Forge's noun, said the way the screen says it: GitHub's *pull request*
//! as it always read, and any other Forge's word with its own article, plural
//! and capitals (ADR-0030, *Forge*).

import { describe, expect, it } from "vitest";

import type { RepoEntry } from "../src/api/types";
import { aNoun, headed, nounOf, opening, spoken } from "../src/forges";
import { targetPlaceholder } from "../src/workbench/Setup";

const ON: RepoEntry = {
  id: 1,
  name: "widgets",
  path: "/srv/widgets",
  default_branch: "main",
  author: null,
  author_unpicked: false,
  github: null,
  origin: null,
  forge: {
    kind: "On",
    uuid: "x",
    name: "GitLab",
    noun: "merge request",
    capabilities: ["checks", "review", "publish", "stacks", "create"],
  },
};

describe("a forge's noun", () => {
  it("is the forge's on a repo on one, and pull request otherwise", () => {
    expect(nounOf(ON)).toBe("merge request");
    expect(nounOf({ ...ON, forge: null })).toBe("pull request");
    expect(nounOf({ ...ON, forge: { kind: "Unmatched", host: "x.org" } })).toBe(
      "pull request",
    );
  });

  it("is headed, opened and given its article", () => {
    expect(headed("merge request")).toBe("Merge Request");
    expect(opening("merge request")).toBe("Merge request");
    expect(aNoun("merge request")).toBe("a merge request");
    expect(aNoun("issue")).toBe("an issue");
  });

  it("says a sentence about a pull request in the forge's word", () => {
    expect(
      spoken(
        "A pull request is open. This work has no pull request. Pull requests wait.",
        "merge request",
      ),
    ).toBe(
      "A merge request is open. This work has no merge request. Merge requests wait.",
    );
    expect(spoken("a pull request", "issue")).toBe("an issue");
  });

  it("leaves GitHub's sentences exactly as they were", () => {
    const said = "A pull request is open. Pull requests wait.";
    expect(spoken(said, "pull request")).toBe(said);
    expect(targetPlaceholder("pull request")).toBe("Pull request or branch");
  });

  it("names the target field's placeholder in the forge's word", () => {
    expect(targetPlaceholder(nounOf(ON))).toBe("Merge request or branch");
  });
});
