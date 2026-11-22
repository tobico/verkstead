//! What the viewer draws around the roles a Process declares: the row a role's
//! picker offers above the pairings, which shape the one **Agent** control
//! takes, what a Conversation settled for a role, and the words an inert start
//! counts off the roles.
//!
//! **Which roles a Process has is not here**: its manifest declares them and
//! the server sends them with it (ADR-0030, *Process*) — see `provided.ts`,
//! and `provided.test.ts`, which is where every word about a Process itself is
//! asked for. What is asked of this module is what Verkstead draws over
//! whatever they turn out to be.
//!
//! Asked of the module rather than of a rendered row, for the reason it is a
//! module at all: it is the one place these words are written down, and a page
//! that drew three pickers because it had three components is not what says a
//! Develop declares three roles.

import { describe, expect, it } from "vitest";

import type { RoleEntry } from "../src/api/types";
import {
  away,
  control,
  label,
  needed,
  roles,
  settledFor,
} from "../src/workbench/roles";

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

  /// And the whole sentence, which is the clauses the caller asked for: a
  /// Review's two roles and the target it is pointed at, a Develop's three
  /// roles and no target.
  it("names a target where the caller says it is wanted", () => {
    expect(needed(2, { brief: true, target: true })).toBe(
      "a brief, a target, and both roles picked and working",
    );
    expect(needed(3, { brief: true, target: false })).toBe(
      "a brief, and every role picked and working",
    );
  });

  /// A **Fix Merge Issues** waits on the same three things a Review does and
  /// names one role rather than two, nothing about it ever reading the branch:
  /// the clause is counted off its roles, so this is the sentence saying so.
  it("names one role for a fix merge issues, and its target with it", () => {
    expect(needed(1, { brief: true, target: true })).toBe(
      "a brief, a target, and one role picked and working",
    );
  });

  /// And leaves out what the caller says is answered already: a page holding a
  /// roadmap has its brief, and one holding a pull request has its target.
  it("leaves out the clauses the caller does not ask for", () => {
    expect(needed(3, { brief: false, target: false })).toBe(
      "every role picked and working",
    );
    expect(needed(2, { brief: true, target: false })).toBe(
      "a brief, and both roles picked and working",
    );
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

