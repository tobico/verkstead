//! Every word the viewer says about a Process, and about the state a
//! Conversation stands in, read off the list the server sends.
//!
//! **There is no table of Processes or states in the viewer** (ADR-0030,
//! *Process*; Set 1765, Q29). A Process is data in a plugin's manifest — the
//! five Verkstead ships are bundled plugins of their own — so its name, the
//! sentence under its picker row, whether the picker offers it, whether it
//! takes a Target, whether its wrap-up is narrowed and the word each of its
//! states is said in all come off `GET /api/ui/processes`.
//!
//! So what is asked here is that each reading goes to that list and comes back
//! with what the server put in it: the fixture is what the endpoint actually
//! answered — see `processes.ts` — and a built-in's manifest reworded on the
//! server's side arrives here as this file failing rather than as two lists
//! quietly disagreeing.

import { describe, expect, it } from "vitest";

import type { ProcessEntry } from "../src/api/types";
import {
  defaultChoice,
  filledFromAPullRequest,
  knowProvidedProcesses,
  narrowed,
  offeredProcesses,
  processDescription,
  interviews,
  processName,
  stateWord,
  targeted,
} from "../src/workbench/provided";
import {
  choice,
  described,
  named,
  offeringOnly,
  PROCESSES,
  whose,
} from "./processes";

/// A plugin's own Process, as the server lists one: two states of its own
/// words, pointed at nothing.
const THEIRS: ProcessEntry = {
  plugin: "5e0c9a7e-2f4b-4c1d-8a3e-6b9f0d2c7a41",
  id: "two-steps",
  name: "Two steps",
  description: "Draft the change, then check it.",
  offered: true,
  states: [
    { id: "drafting", word: "Drafting", interviews: true },
    { id: "checking", word: "Checking", interviews: false },
  ],
  roles: [
    { role: "sketching", label: "Sketching", building: true, away: false },
  ],
  target: false,
  narrowed: false,
  filled_from_a_pull_request: false,
};

/// Which Process a Conversation on `THEIRS` names.
const WHOSE = { plugin: THEIRS.plugin, id: THEIRS.id };

/// And a Process no plugin here provides: one removed under a Conversation.
const GONE = { plugin: "gone", id: "triage" };

describe("what each process is called", () => {
  /// The name its manifest gives it, for one of Verkstead's own as for a
  /// plugin's: the five are plugins too, so there is one rule rather than a
  /// table and a rule.
  it("is the name its manifest gives it", () => {
    expect(processName(whose("develop"))).toBe(named("develop"));
    expect(processName(whose("fix-merge-issues"))).toBe("Fix merge issues");
  });

  it("is a plugin's own name where the Conversation is on one", () => {
    knowProvidedProcesses([...PROCESSES, THEIRS]);

    expect(processName(WHOSE)).toBe("Two steps");
  });

  /// And a Process this page holds nothing for is named by what it names: a
  /// plugin removed under a Conversation, which a blank would say nothing
  /// about.
  it("is the id where no such process is listed", () => {
    expect(processName(GONE)).toBe("triage");
  });
});

describe("the sentence under a picker row", () => {
  /// Its manifest's description, which is the one the human wrote.
  it("is the description the server sent", () => {
    expect(processDescription(choice("review"))).toBe(described("review"));
    expect(processDescription(choice("tinker"))).toBe(described("tinker"));
  });

  it("is a plugin's own where the row is a plugin's", () => {
    knowProvidedProcesses([...PROCESSES, THEIRS]);

    expect(processDescription({ process: WHOSE })).toBe(
      "Draft the change, then check it.",
    );
  });

  /// Every row the picker offers has one: a row offered with no sentence under
  /// it would be the one row of the list that left the human guessing.
  it("is said under every row offered", () => {
    knowProvidedProcesses([...PROCESSES, THEIRS]);

    for (const offered of offeredProcesses()) {
      expect(processDescription(offered)).not.toBe("");
    }
  });
});

describe("which processes the picker offers", () => {
  /// The ones the server says are offered, in the order it listed them:
  /// Verkstead's own five first, then each plugin's.
  it("is the server's list in the server's order", () => {
    knowProvidedProcesses([...PROCESSES, THEIRS]);

    expect(offeredProcesses()).toEqual([
      choice("develop"),
      choice("tinker"),
      choice("investigate"),
      choice("review"),
      choice("fix-merge-issues"),
      { process: WHOSE },
    ]);
  });

  /// A plugin unticked takes its Process off the picker and leaves the rest
  /// on it — one of Verkstead's own included, every one of the five being a
  /// plugin with a tick.
  it("leaves off a process whose plugin is unticked", () => {
    knowProvidedProcesses(offeringOnly(["develop", "investigate", "review"]));

    expect(offeredProcesses()).toEqual([
      choice("develop"),
      choice("investigate"),
      choice("review"),
    ]);
  });

  /// And the row an untouched picker stands on is the first of them, which is
  /// Develop while its tick stands and the next one offered where it does not.
  it("stands on the first process offered", () => {
    expect(defaultChoice()).toEqual(choice("develop"));

    knowProvidedProcesses(offeringOnly(["investigate", "review"]));
    expect(defaultChoice()).toEqual(choice("investigate"));
  });

  /// And on nothing at all where nothing is offered, which is every tick off:
  /// the picker draws no row rather than naming a Process nobody offered, and
  /// the record is left reading Develop for Start to refuse.
  it("stands on nothing where nothing is offered", () => {
    knowProvidedProcesses(offeringOnly([]));

    expect(defaultChoice()).toBeNull();
  });
});

describe("which process a pull request fills the picker with", () => {
  /// **Fix Merge Issues**, which is what the server's own fill writes — so the
  /// composer draws the row the record is about to hold rather than keeping a
  /// word of its own for which Process that is.
  it("is the one the server marks", () => {
    const filled = PROCESSES.filter(
      (entry) => entry.filled_from_a_pull_request,
    ).map((entry) => entry.id);

    expect(filled).toEqual(["fix-merge-issues"]);
    expect(filledFromAPullRequest()).toEqual(choice("fix-merge-issues"));
  });

  /// And nothing where the list holds no such row, which is its plugin
  /// unticked or gone: the picker then stays where it was.
  it("is nothing where no row is marked", () => {
    knowProvidedProcesses([THEIRS]);

    expect(filledFromAPullRequest()).toBeNull();
  });
});

describe("which processes are pointed at a target", () => {
  /// **Review** and **Fix Merge Issues**, which are the two wrap-ups over work
  /// already somewhere else — and it is their manifests saying so, so a
  /// Process that declares a target draws the field without a line changing in
  /// either composer.
  it("is what each manifest says", () => {
    const pointed = PROCESSES.filter((entry) => entry.target).map(
      (entry) => entry.id,
    );

    expect(pointed).toEqual(["review", "fix-merge-issues"]);

    for (const entry of PROCESSES) {
      expect(targeted(whose(entry.id))).toBe(entry.target);
    }
  });

  /// And a Process this page holds nothing for draws no field: there is
  /// nothing saying it wants one.
  it("draws no field for a process nothing is listed for", () => {
    expect(targeted(GONE)).toBe(false);
  });
});

describe("which processes wrap up narrowed", () => {
  /// **Fix Merge Issues** and nothing else: its wrapping state turns the
  /// Wrap-up's review and comments off, which is the whole of the narrowing,
  /// and the Steer form's note is drawn off the same fact.
  it("is what each manifest's wrapping state says", () => {
    const narrows = PROCESSES.filter((entry) => entry.narrowed).map(
      (entry) => entry.id,
    );

    expect(narrows).toEqual(["fix-merge-issues"]);

    for (const entry of PROCESSES) {
      expect(narrowed(whose(entry.id))).toBe(entry.narrowed);
    }
  });
});

describe("the word a state is said in", () => {
  /// Its Process's, by the word the record holds — a Grilling carries
  /// `grilling` and reads *Grilling* off Develop's manifest, and *Follow-up* is
  /// the hyphenated word the manifest spells rather than the id.
  it("is the word its process declares", () => {
    expect(stateWord(whose("develop"), "grilling")).toBe("Grilling");
    expect(stateWord(whose("develop"), "follow-up")).toBe("Follow-up");
    expect(stateWord(whose("tinker"), "wrapping")).toBe("Wrapping");
  });

  /// A state a plugin's Process declares reads whatever that plugin called it.
  it("is a plugin's own word for a state it declares", () => {
    knowProvidedProcesses([...PROCESSES, THEIRS]);

    expect(stateWord(WHOSE, "checking")).toBe("Checking");
  });

  /// And a word nothing declares — a plugin removed under a Conversation — is
  /// drawn as the word the record holds, which is legible where a blank would
  /// not be.
  it("is the word itself where no process is listed for it", () => {
    expect(stateWord(GONE, "sorting")).toBe("sorting");
  });

  /// The three Verkstead words itself are Verkstead's own words whatever the
  /// Process: nothing returns to any of them, and no Process declares one.
  it("is Verkstead's own for Draft, Done and Closed", () => {
    expect(stateWord(whose("develop"), "draft")).toBe("Draft");
    expect(stateWord(whose("develop"), "done")).toBe("Done");
    expect(stateWord(whose("develop"), "closed")).toBe("Closed");
  });

  /// And which state runs the interview, the one thing besides the word the
  /// page asks of a state: a steer into it opens a round, and the digest tick
  /// on its form primes that round's interview.
  it("says which state runs the interview", () => {
    expect(interviews(whose("develop"), "grilling")).toBe(true);
    expect(interviews(whose("develop"), "implementing")).toBe(false);
    expect(interviews(whose("tinker"), "follow-up")).toBe(false);
    expect(interviews(GONE, "sorting")).toBe(false);
  });
});
