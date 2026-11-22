//! What the viewer draws around a Process and the roles it declares: the words
//! each Process is said in, which of them the picker offers, which are pointed
//! at a target, the row a role's picker offers above the pairings, which shape
//! the one **Agent** control takes, and the words an inert start counts off the
//! roles.
//!
//! **Which roles a Process has is not here**: its manifest declares them and
//! the server sends them with it (ADR-0030, *Process*) — see `provided.ts`.
//! What is asked of this module is what Verkstead draws over whatever they
//! turn out to be.
//!
//! Asked of the module rather than of a rendered row, for the reason it is a
//! module at all: it is the one place these words are written down, and a page
//! that drew three pickers because it had three components is not what says a
//! Develop declares three roles.

import { describe, expect, it } from "vitest";

import type { Process, RoleEntry } from "../src/api/types";
import {
  away,
  control,
  DESCRIPTION,
  label,
  narrowed,
  NARROWED,
  needed,
  OFFERED,
  PROCESS,
  roles,
  settledFor,
  TARGETED,
  targeted,
} from "../src/workbench/processes";

/// Every Process the wire carries, which is every word the viewer has for one.
/// The five Verkstead runs itself. A plugin's is worded by its manifest —
/// see `provided.ts`.
const EVERY = (Object.keys(PROCESS) as Process[]).filter(
  (process) => process !== "Provided",
);

/// One role as a manifest declares it, for the readings below.
function role(
  name: string,
  label: string,
  marks: { building?: boolean; away?: boolean } = {},
): RoleEntry {
  return {
    role: name,
    label,
    building: marks.building ?? false,
    away: marks.away ?? false,
  };
}

describe("the row a role's picker offers above the pairings", () => {
  /// A role whose manifest marks it `away` offers the row that is no account at
  /// all, worded off its label: *No review*, and whatever a plugin's own role
  /// calls it.
  it("is worded off the role's own label", () => {
    expect(away(role("review", "Review", { away: true }))).toBe("No review");
    expect(away(role("checking", "Checking", { away: true }))).toBe(
      "No checking",
    );
  });

  /// And a role without it has to be answered with an account: there is no row
  /// to offer, so there is nothing to word.
  it("is nothing for a role that has to be answered", () => {
    expect(away(role("grilling", "Grilling"))).toBeUndefined();
    expect(
      away(role("implementation", "Implementation", { building: true })),
    ).toBeUndefined();
  });
});

describe("which shape the one Agent control takes", () => {
  /// Counted off the Process's roles rather than read out of a table: a panel
  /// where there are several pickers to stack under their labels, and the flat
  /// picker where there is one. A panel over one picker would be a press to
  /// reach a press.
  it("is a panel for several roles and the picker itself for one", () => {
    expect(control(3)).toBe("panel");
    expect(control(2)).toBe("panel");
    expect(control(1)).toBe("dropdown");
  });

  /// And a Process this page holds no entry for has no roles to draw, which
  /// falls to the one picker rather than to an empty panel.
  it("is the picker itself where there are no roles to draw", () => {
    expect(control(0)).toBe("dropdown");
  });
});

describe("what a conversation settled for a role", () => {
  /// By name, off the list the record sends: whichever roles its Process
  /// declares, and nothing for one it declares none of.
  it("is read by the role's name", () => {
    const pairings = [
      { ...role("implementation", "Implementation", { building: true }), picked: "Nothing" as const },
      { ...role("review", "Review", { away: true }), picked: "Skipped" as const },
    ];

    expect(settledFor(pairings, "review")).toBe("Skipped");
    expect(settledFor(pairings, "implementation")).toBe("Nothing");
    expect(settledFor(pairings, "grilling")).toBe("Nothing");
  });
});

describe("what an inert start says it is waiting on", () => {
  /// Counted off the Process's own roles rather than fixed at three: the
  /// sentence written for a held pull request is the two-role sentence, and
  /// which roles there are is the manifest's.
  it("names the roles as the process has them", () => {
    expect(roles(3)).toBe("every role");
    expect(roles(2)).toBe("both roles");
    expect(roles(1)).toBe("one role");
  });

  it("says something however many roles there are", () => {
    for (const count of [0, 1, 2, 3, 4]) {
      expect(roles(count)).not.toBe("");
    }
  });

  /// And the whole sentence, which is the clauses the caller asked for with
  /// the target added where the Process takes one.
  it("names a target where the process is pointed at one", () => {
    expect(needed("Review", 2, { brief: true, target: true })).toBe(
      "a brief, a target, and both roles picked and working",
    );
    expect(needed("Develop", 3, { brief: true, target: true })).toBe(
      "a brief, and every role picked and working",
    );
  });

  /// A **Fix Merge Issues** waits on the same three things a Review does and
  /// names one role rather than two, nothing about it ever reading the branch:
  /// the clause is counted off its roles, so this is the sentence saying so.
  it("names one role for a fix merge issues, and its target with it", () => {
    expect(needed("FixMergeIssues", 1, { brief: true, target: true })).toBe(
      "a brief, a target, and one role picked and working",
    );
  });

  /// And leaves out what the caller says is answered already: a page holding a
  /// roadmap has its brief, and one holding a pull request has its target.
  it("leaves out the clauses the caller does not ask for", () => {
    expect(needed("Develop", 3, { brief: false, target: true })).toBe(
      "every role picked and working",
    );
    expect(needed("Review", 2, { brief: true, target: false })).toBe(
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

describe("what a role's picker is labelled", () => {
  /// Inside a panel it is the role's own label, as its manifest writes it:
  /// several pickers stacked, and a label apiece is the whole of what tells
  /// them apart.
  it("names the role where the control is a panel", () => {
    expect(label(3, role("grilling", "Grilling"))).toBe("Grilling");
    expect(label(3, role("implementation", "Implementation"))).toBe(
      "Implementation",
    );
    expect(label(2, role("checking", "Checking"))).toBe("Checking");
  });

  /// And where there is one role there is no panel to tell anything apart in:
  /// the picker *is* the one **Agent** control, so it wears the row's own
  /// label.
  it("says Agent where the control is the picker itself", () => {
    expect(label(1, role("implementation", "Implementation"))).toBe("Agent");
    expect(label(1, role("sketching", "Sketching"))).toBe("Agent");
  });

  /// One or the other whatever the role: a picker with no label is a control a
  /// screen reader cannot name.
  it("labels a picker whichever shape draws it", () => {
    for (const count of [1, 2, 3]) {
      expect(label(count, role("drawing", "Drawing"))).not.toBe("");
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

describe("what each process is said to be", () => {
  /// A sentence for every Process the wire carries, for the reason `PROCESS`
  /// has a word for each: the picker draws one under every row, and a row with
  /// none would be the one the human had to guess at.
  it("has a sentence for each of the five", () => {
    expect(Object.keys(DESCRIPTION).sort()).toEqual([...EVERY].sort());

    for (const process of EVERY) {
      expect(DESCRIPTION[process]).not.toBe("");
    }
  });

  /// In the human's own words, *PR* and all.
  it("says them the way the human wrote them", () => {
    expect(DESCRIPTION).toEqual({
      Develop:
        "Work on something complex, starting with a grilling session and proceeding through a step-by-step development process",
      Tinker: "Make a series of simple one-off changes with minimal fuss",
      Investigate:
        "Ask questions about a code base without committing any code",
      Review: "Review an existing PR or branch, and fix any issues found",
      FixMergeIssues: "Fix up a PR which is unable to merge",
    });
  });
});
