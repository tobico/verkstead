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

import type { Process, ProcessEntry } from "../src/api/types";
import {
  defaultChoice,
  knowProvidedProcesses,
  narrowed,
  offeredProcesses,
  processDescription,
  processName,
  rowWord,
  stateWord,
  targeted,
} from "../src/workbench/provided";
import { described, named, offeringOnly, PROCESSES } from "./processes";

/// A plugin's own Process, as the server lists one: two states of its own
/// words, pointed at nothing.
const THEIRS: ProcessEntry = {
  plugin: "5e0c9a7e-2f4b-4c1d-8a3e-6b9f0d2c7a41",
  id: "two-steps",
  name: "Two steps",
  description: "Draft the change, then check it.",
  offered: true,
  states: [
    { id: "drafting", word: "Drafting" },
    { id: "checking", word: "Checking" },
  ],
  roles: [
    { role: "sketching", label: "Sketching", building: true, away: false },
  ],
  target: false,
  narrowed: false,
  built_in: null,
};

/// Which plugin's Process a Conversation on `THEIRS` names.
const WHOSE = { plugin: THEIRS.plugin, id: THEIRS.id };

describe("what each process is called", () => {
  /// The name its manifest gives it, for one of Verkstead's own as for a
  /// plugin's: the five are plugins too, so there is one rule rather than a
  /// table and a rule.
  it("is the name its manifest gives it", () => {
    expect(processName("Develop", null)).toBe(named("Develop"));
    expect(processName("FixMergeIssues", null)).toBe("Fix merge issues");
  });

  it("is a plugin's own name where the Conversation is on one", () => {
    knowProvidedProcesses([...PROCESSES, THEIRS]);

    expect(processName("Provided", WHOSE)).toBe("Two steps");
  });

  /// And a Process this page holds nothing for is named by what it names: a
  /// plugin removed under a Conversation, which a blank would say nothing
  /// about.
  it("is the id where no such process is listed", () => {
    expect(processName("Provided", { plugin: "gone", id: "triage" })).toBe(
      "triage",
    );
  });
});

describe("the sentence under a picker row", () => {
  /// Its manifest's description, which is the one the human wrote.
  it("is the description the server sent", () => {
    expect(processDescription({ process: "Review" })).toBe(described("Review"));
    expect(processDescription({ process: "Tinker" })).toBe(described("Tinker"));
  });

  it("is a plugin's own where the row is a plugin's", () => {
    knowProvidedProcesses([...PROCESSES, THEIRS]);

    expect(
      processDescription({ process: "Provided", provided: WHOSE }),
    ).toBe("Draft the change, then check it.");
  });

  /// Every row the picker offers has one: a row offered with no sentence under
  /// it would be the one row of the list that left the human guessing.
  it("is said under every row offered", () => {
    knowProvidedProcesses([...PROCESSES, THEIRS]);

    for (const choice of offeredProcesses()) {
      expect(processDescription(choice)).not.toBe("");
    }
  });
});

describe("which processes the picker offers", () => {
  /// The ones the server says are offered, in the order it listed them:
  /// Verkstead's own five first, then each plugin's.
  it("is the server's list in the server's order", () => {
    knowProvidedProcesses([...PROCESSES, THEIRS]);

    expect(offeredProcesses().map((choice) => choice.process)).toEqual([
      "Develop",
      "Tinker",
      "Investigate",
      "Review",
      "FixMergeIssues",
      "Provided",
    ]);
  });

  /// A plugin unticked takes its Process off the picker and leaves the rest
  /// on it — one of Verkstead's own included, every one of the five being a
  /// plugin with a tick.
  it("leaves off a process whose plugin is unticked", () => {
    knowProvidedProcesses(offeringOnly(["Develop", "Investigate", "Review"]));

    expect(offeredProcesses().map((choice) => choice.process)).toEqual([
      "Develop",
      "Investigate",
      "Review",
    ]);
  });

  /// And the row an untouched picker stands on is the first of them, which is
  /// Develop while its tick stands and the next one offered where it does not.
  it("stands on the first process offered", () => {
    expect(defaultChoice()).toEqual({ process: "Develop" });

    knowProvidedProcesses(offeringOnly(["Investigate", "Review"]));
    expect(defaultChoice()).toEqual({ process: "Investigate" });
  });

  /// Develop where nothing at all is offered, which is every tick off: the
  /// record reads Develop with no row of its own, and Start says it is not
  /// offered rather than the page naming something nobody offered.
  it("stands on Develop where nothing is offered", () => {
    knowProvidedProcesses(offeringOnly([]));

    expect(defaultChoice()).toEqual({ process: "Develop" });
  });
});

describe("which processes are pointed at a target", () => {
  /// **Review** and **Fix Merge Issues**, which are the two wrap-ups over work
  /// already somewhere else — and it is their manifests saying so, so a
  /// Process that declares a target draws the field without a line changing in
  /// either composer.
  it("is what each manifest says", () => {
    const pointed = PROCESSES.filter((entry) => entry.target).map(
      (entry) => entry.built_in,
    );

    expect(pointed).toEqual(["Review", "FixMergeIssues"]);

    for (const entry of PROCESSES) {
      expect(targeted(entry.built_in as Process, null)).toBe(entry.target);
    }
  });

  /// And a Process this page holds nothing for draws no field: there is
  /// nothing saying it wants one.
  it("draws no field for a process nothing is listed for", () => {
    expect(targeted("Provided", { plugin: "gone", id: "triage" })).toBe(false);
  });
});

describe("which processes wrap up narrowed", () => {
  /// **Fix Merge Issues** and nothing else: its wrapping state turns the
  /// Wrap-up's review and comments off, which is the whole of the narrowing,
  /// and the Steer form's note is drawn off the same fact.
  it("is what each manifest's wrapping state says", () => {
    const narrows = PROCESSES.filter((entry) => entry.narrowed).map(
      (entry) => entry.built_in,
    );

    expect(narrows).toEqual(["FixMergeIssues"]);

    for (const entry of PROCESSES) {
      expect(narrowed(entry.built_in as Process, null)).toBe(entry.narrowed);
    }
  });
});

describe("the word a state is said in", () => {
  /// Its Process's, by the id the server sends beside it — a Grilling reads
  /// *Grilling* off Develop's manifest, and *Follow-up* is the hyphenated word
  /// the manifest spells rather than the variant's.
  it("is the word its process declares", () => {
    expect(stateWord("Develop", null, "Grilling", "grilling")).toBe("Grilling");
    expect(stateWord("Develop", null, "FollowUp", "follow-up")).toBe(
      "Follow-up",
    );
    expect(stateWord("Tinker", null, "Wrapping", "wrapping")).toBe("Wrapping");
  });

  /// A state a plugin's Process declares reads whatever that plugin called it.
  it("is a plugin's own word for a state it declares", () => {
    knowProvidedProcesses([...PROCESSES, THEIRS]);

    expect(stateWord("Provided", WHOSE, "Declared", "checking")).toBe(
      "Checking",
    );
  });

  /// And an unknown word — a plugin removed under a Conversation — is drawn as
  /// the word the record holds, which is legible where a blank would not be.
  it("is the word itself where no process is listed for it", () => {
    expect(
      stateWord("Provided", { plugin: "gone", id: "triage" }, "Declared", "sorting"),
    ).toBe("sorting");
  });

  /// The three Verkstead words itself carry no state id and are Verkstead's
  /// own words: nothing returns to any of them, and no Process declares one.
  it("is Verkstead's own for Draft, Done and Closed", () => {
    expect(stateWord("Develop", null, "Draft", null)).toBe("Draft");
    expect(stateWord("Develop", null, "Done", null)).toBe("Done");
    expect(stateWord("Develop", null, "Closed", null)).toBe("Closed");
  });

  /// And a sidebar row reads the same word off the plugin its row names, that
  /// being what the server sends it as.
  it("is read the same way on a sidebar row", () => {
    expect(
      rowWord(
        { process: { plugin: PROCESSES[0]!.plugin, id: "develop" }, state: "grilling" },
        "Grilling",
      ),
    ).toBe("Grilling");
    expect(rowWord(null, "Draft")).toBe("Draft");
  });
});
