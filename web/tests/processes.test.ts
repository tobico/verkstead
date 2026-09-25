//! The role table, away from the two composers that draw off it: which roles
//! each Process uses, which of them may be picked away, which shape its control
//! takes, and the words an inert start counts off it.
//!
//! Asked of the module rather than of a rendered row, for the reason it is a
//! module at all: it is the one place the table is written down, and a page
//! that drew three pickers because it had three components is not what says a
//! Develop uses three roles.

import { describe, expect, it } from "vitest";

import type { Process } from "../src/api/types";
import {
  OFFERED,
  PROCESS,
  ROLES,
  roles,
  uses,
} from "../src/workbench/processes";
import type { Role } from "../src/workbench/processes";

/// Every Process the wire carries, which is every word the viewer has for one.
const EVERY = Object.keys(PROCESS) as Process[];

describe("the roles a process is run under", () => {
  /// The table ADR-0020 carries, row for row — and Develop's row is the one
  /// every draft gets while it is the only Process offered.
  it("says what each of the five uses", () => {
    expect(ROLES.Develop.uses).toEqual([
      "grilling",
      "implementation",
      "review",
    ]);
    expect(ROLES.Investigate.uses).toEqual(["implementation"]);
    expect(ROLES.Review.uses).toEqual(["implementation", "review"]);
    expect(ROLES.Tinker.uses).toEqual(["implementation", "review"]);
    expect(ROLES.FixMergeIssues.uses).toEqual(["implementation"]);
  });

  /// And which of them the human may answer with nobody at all. A Review
  /// without a review is Fix Merge Issues with the comments answered, which is
  /// a different thing to ask for — so its Review role is not one of them.
  it("says which of them may be picked away", () => {
    expect(ROLES.Develop.away).toEqual(["review"]);
    expect(ROLES.Tinker.away).toEqual(["review"]);
    expect(ROLES.Review.away).toEqual([]);
    expect(ROLES.Investigate.away).toEqual([]);
    expect(ROLES.FixMergeIssues.away).toEqual([]);
  });

  /// A role nothing draws cannot be picked away from, so every row's `away` is
  /// a part of its `uses` rather than a list beside it.
  it("names nothing it does not also use", () => {
    for (const process of EVERY) {
      for (const role of ROLES[process].away) {
        expect(ROLES[process].uses).toContain(role);
      }
    }
  });

  /// The shape the one **Agent** control takes: a panel where there are several
  /// pickers to stack under their labels, and the flat Pairing dropdown where
  /// there is one.
  it("takes a panel for several roles and a dropdown for one", () => {
    for (const process of EVERY) {
      expect(ROLES[process].control).toBe(
        ROLES[process].uses.length === 1 ? "dropdown" : "panel",
      );
    }

    // With only Develop landed, the panel is the shape every draft gets.
    expect(OFFERED).toEqual(["Develop"]);
    expect(ROLES.Develop.control).toBe("panel");
  });

  /// Every Process the wire carries has a row, for the reason all five are
  /// worded: a record naming one this viewer had no roles for would be a
  /// composer with nothing in its setup row.
  it("has a row for every process there is a word for", () => {
    expect(Object.keys(ROLES).sort()).toEqual(EVERY.sort());
  });

  /// Nothing runs without something building it, whichever Process it is.
  it("uses the implementation role whatever the process", () => {
    for (const process of EVERY) {
      expect(uses(process, "implementation")).toBe(true);
    }
  });

  it("answers whether a process uses a role", () => {
    expect(uses("Develop", "grilling")).toBe(true);
    // The one this stage moves off a test for a held pull request: a
    // Conversation holding one reads as a Review, and a Review has no round for
    // a grilling to open.
    expect(uses("Review", "grilling")).toBe(false);
    expect(uses("Investigate", "review")).toBe(false);
  });
});

describe("what an inert start says it is waiting on", () => {
  /// Counted off the table rather than fixed at three: the sentence written for
  /// a held pull request is the two-role sentence, and it is the table's now.
  it("names the roles as the process has them", () => {
    expect(roles("Develop")).toBe("every role");
    expect(roles("Review")).toBe("both roles");
    expect(roles("Tinker")).toBe("both roles");
    expect(roles("Investigate")).toBe("one role");
    expect(roles("FixMergeIssues")).toBe("one role");
  });

  it("says something for every process there is", () => {
    for (const process of EVERY) {
      expect(roles(process)).not.toBe("");
    }
  });
});

describe("the roles themselves", () => {
  /// Spelled the way the record's own fields spell them — `grilling_pairing`
  /// and the two beside it — because a composer indexes what it is holding by
  /// exactly these words.
  it("are the three the record keys its pairings by", () => {
    const every: Role[] = ["grilling", "implementation", "review"];
    const drawn = new Set(EVERY.flatMap((process) => ROLES[process].uses));

    expect([...drawn].sort()).toEqual([...every].sort());
  });
});
