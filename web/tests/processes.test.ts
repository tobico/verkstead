//! The role table, away from the two composers that draw off it: which roles
//! each Process uses, which of them may be picked away, which shape its control
//! takes, which of them are pointed at a target, what each of their pickers is
//! labelled, and the words an inert start counts off it.
//!
//! Asked of the module rather than of a rendered row, for the reason it is a
//! module at all: it is the one place the table is written down, and a page
//! that drew three pickers because it had three components is not what says a
//! Develop uses three roles.

import { describe, expect, it } from "vitest";

import type { Process } from "../src/api/types";
import {
  away,
  label,
  narrowed,
  NARROWED,
  needed,
  OFFERED,
  PROCESS,
  ROLE,
  ROLES,
  roles,
  TARGETED,
  targeted,
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

  /// And which of them the human may answer with nobody at all, which is what
  /// each picker's own rows are drawn from. Two of the five offer one: Develop
  /// and Tinker, over the review. A **Review** offers none — a Review without a
  /// review is Fix Merge Issues with the comments answered — and *No grilling*
  /// has gone already.
  it("says which of them may be picked away", () => {
    expect(ROLES.Develop.away).toEqual(["review"]);
    expect(ROLES.Tinker.away).toEqual(["review"]);
    expect(ROLES.Review.away).toEqual([]);
    expect(ROLES.Investigate.away).toEqual([]);
    expect(ROLES.FixMergeIssues.away).toEqual([]);
  });

  /// The implementation role is on no row's list, there being no work without
  /// something building it — which is the same fact as every row using it.
  it("never offers the implementation role a row that runs nothing", () => {
    for (const process of EVERY) {
      expect(ROLES[process].away).not.toContain("implementation");
      expect(away(process, "implementation")).toBeUndefined();
    }
  });

  /// And the words the row is said in, which is what a composer asks for rather
  /// than writing them in itself: a row a picker spelled by hand would go on
  /// being offered through the stage that retired it.
  it("says the words each row that runs nothing is offered in", () => {
    expect(away("Develop", "review")).toBe("No review");
    expect(away("Tinker", "review")).toBe("No review");
  });

  /// And nothing where the role has to be answered with an account — the
  /// grilling role on every Process that uses it, *No grilling* being retired.
  it("offers no row where the table does not name the role", () => {
    // The review on a Review among them: it is the one role that Process draws
    // beside the implementation, and it has to be answered with an account.
    expect(away("Review", "review")).toBeUndefined();
    expect(away("Investigate", "review")).toBeUndefined();
    expect(away("FixMergeIssues", "review")).toBeUndefined();
    expect(away("Develop", "grilling")).toBeUndefined();
    expect(away("Tinker", "grilling")).toBeUndefined();
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

    // And which of them a draft can be, which is all five now: three run under
    // several roles and draw the panel, and the Investigate and the Fix Merge
    // Issues are the two that draw the dropdown.
    expect(OFFERED).toEqual([
      "Develop",
      "Tinker",
      "Investigate",
      "Review",
      "FixMergeIssues",
    ]);
    expect(ROLES.Develop.control).toBe("panel");
    expect(ROLES.Tinker.control).toBe("panel");
    expect(ROLES.Review.control).toBe("panel");
    expect(ROLES.Investigate.control).toBe("dropdown");
    expect(ROLES.FixMergeIssues.control).toBe("dropdown");
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

  /// And the whole sentence, which is the clauses the caller asked for with
  /// the target added where the table says the Process takes one.
  it("names a target where the process is pointed at one", () => {
    expect(needed("Review", { brief: true, target: true })).toBe(
      "a brief, a target, and both roles picked and working",
    );
    expect(needed("Develop", { brief: true, target: true })).toBe(
      "a brief, and every role picked and working",
    );
  });

  /// A **Fix Merge Issues** waits on the same three things a Review does and
  /// names one role rather than two, nothing about it ever reading the branch:
  /// the clause is counted off the table, so this is the row saying so.
  it("names one role for a fix merge issues, and its target with it", () => {
    expect(needed("FixMergeIssues", { brief: true, target: true })).toBe(
      "a brief, a target, and one role picked and working",
    );
  });

  /// And leaves out what the caller says is answered already: a page holding a
  /// roadmap has its brief, and one holding a pull request has its target.
  it("leaves out the clauses the caller does not ask for", () => {
    expect(needed("Develop", { brief: false, target: true })).toBe(
      "every role picked and working",
    );
    expect(needed("Review", { brief: true, target: false })).toBe(
      "a brief, and both roles picked and working",
    );
  });
});

describe("the processes that are pointed at a target", () => {
  /// **Review** and **Fix Merge Issues**, which are the two wrap-ups over work
  /// already somewhere else: the field in the Repo panel comes out of this
  /// list, so a Process that gains a target gains the field without a line
  /// changing in either composer — and so does the press that takes it up.
  it("is the two wrap-ups over work already somewhere else", () => {
    expect(TARGETED).toEqual(["Review", "FixMergeIssues"]);

    for (const process of EVERY) {
      expect(targeted(process)).toBe(
        process === "Review" || process === "FixMergeIssues",
      );
    }
  });

  /// Every one of them is offered, which is the pair of lists agreeing: a
  /// Process with a Target field and no row to pick it on would be a field
  /// nobody could reach.
  it("names only processes the picker offers", () => {
    for (const process of TARGETED) {
      expect(OFFERED).toContain(process);
    }
  });

  /// And every one of them is a Process the wire carries, for the reason the
  /// role table has a row for each: a word this viewer has never heard of is
  /// nothing a panel could draw a field for.
  it("names only processes there is a word for", () => {
    for (const process of TARGETED) {
      expect(EVERY).toContain(process);
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

describe("what a role's picker is labelled", () => {
  /// Inside a panel it is the role's own name: several pickers stacked, and a
  /// name apiece is the whole of what tells them apart. The tests, the Brief's
  /// setup facts and the Steer form all speak these three.
  it("names the role where the control is a panel", () => {
    expect(label("Develop", "grilling")).toBe("Grilling");
    expect(label("Develop", "implementation")).toBe("Implementation");
    expect(label("Develop", "review")).toBe("Review");
    expect(label("Review", "review")).toBe(ROLE.review);
  });

  /// And where the table says a dropdown there is no panel to tell anything
  /// apart in: the picker *is* the one **Agent** control, so it wears the row's
  /// own label.
  it("says Agent where the control is the picker itself", () => {
    expect(label("Investigate", "implementation")).toBe("Agent");
    expect(label("FixMergeIssues", "implementation")).toBe("Agent");
  });

  /// One or the other for every role every Process draws: a picker with no
  /// label is a control a screen reader cannot name.
  it("labels every picker any process draws", () => {
    for (const process of EVERY) {
      for (const role of ROLES[process].uses) {
        expect(label(process, role)).not.toBe("");
      }
    }
  });
});

describe("the processes whose wrap-up is narrowed", () => {
  /// **Fix Merge Issues** and nothing else: the one Process pointed at a pull
  /// request that has been reviewed and talked about already, so what is left of
  /// wrapping it up is the two things GitHub itself refuses a merge for.
  ///
  /// The viewer keeps this list so that what the human is *told* is about to
  /// happen comes off the same kind of fact as what happens — the Steer form's
  /// wrapping-up note is drawn off it, and a note promising a review on a Process
  /// that never runs one would promise an hour of work that never happens.
  it("is Fix Merge Issues and nothing else", () => {
    expect(NARROWED).toEqual(["FixMergeIssues"]);

    for (const process of EVERY) {
      expect(narrowed(process)).toBe(process === "FixMergeIssues");
    }
  });

  /// And it is one of the two pointed at a target, which is what makes it a
  /// wrap-up over work already somewhere else rather than a pipeline of its own.
  it("narrows only a process that takes a target", () => {
    for (const process of NARROWED) {
      expect(TARGETED).toContain(process);
      expect(OFFERED).toContain(process);
    }
  });
});
