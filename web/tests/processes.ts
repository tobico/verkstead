//! The Processes the viewer's tests stand on: the five Verkstead ships, as the
//! server lists them.
//!
//! `fixtures/processes.json` is what `GET /api/ui/processes` answered on a
//! server with nothing installed, written by the server's own suite — so a
//! test reading a word out of here is reading the word the endpoint said, and
//! a manifest reworded on that side reaches this one as a failing fixture
//! rather than as two lists quietly disagreeing (ADR-0030, *Process*).
//!
//! Seeded for every test in `setup.ts`. What is here is the way a test *names*
//! one of them: the row to press, the sentence under it, the state words.
//!
//! **A Process is named by its id here**, which is the word the record has
//! always held for each of the five — `develop`, `tinker`, `investigate`,
//! `review`, `fix-merge-issues` — and the plugin providing it comes off the
//! list, so no uuid is written into a test.

import type {
  DeclaredState,
  Lifecycle,
  ProcessChoice,
  ProcessEntry,
  ProvidedProcess,
} from "../src/api/types";
import { knowProvidedProcesses, stateWord } from "../src/workbench/provided";
import listed from "./fixtures/processes.json" with { type: "json" };
import { json, whenever, type Answer } from "./serving";

/// The five, in the order the server lists them and so the order the picker
/// draws their rows.
export const PROCESSES = listed as ProcessEntry[];

/// Which of them the picker offers, as a choice each — the five, each while
/// its plugin's tick stands, which is how a server with nothing unticked lists
/// them.
export const OFFERED: ProcessChoice[] = PROCESSES.map((entry) => ({
  process: whose(entry.id),
}));

/// Say that `entries` is what this server lists, both ways: here, where the
/// page reads it, and on the wire, where the shell reads it once for the whole
/// page.
///
/// Both because the shell's own read lands a moment after a test has said what
/// it means — so a test that seeded a list and let the wire answer another
/// would be drawing a page that changed its mind under it. What it hands back
/// is the answer, for whichever `serving` the test is building.
export function saying(entries: ProcessEntry[]): Answer {
  knowProvidedProcesses(entries);

  return whenever("/api/ui/processes", json(entries));
}

/// The entry for one of them, by the id its manifest gives it.
export function processEntry(process: string): ProcessEntry {
  const entry = PROCESSES.find((listed) => listed.id === process);

  if (entry === undefined) {
    throw new Error(`no Process ${process} is listed`);
  }

  return entry;
}

/// And which Process that is, as a record and a press name one: the plugin
/// providing it and its id there.
export function whose(process: string): ProvidedProcess {
  const entry = processEntry(process);

  return { plugin: entry.plugin, id: entry.id };
}

/// And the same as a picker's choice.
export function choice(process: string): ProcessChoice {
  return { process: whose(process) };
}

/// What it is called: the name its manifest gives it, which is the row's label
/// and the word a frozen Brief reads back.
export function named(process: string): string {
  return processEntry(process).name;
}

/// And the sentence under that row.
export function described(process: string): string {
  return processEntry(process).description;
}

/// The word the `conversations.state` column holds for each of Verkstead's own
/// states, which is the id the five give that state in their manifests — and
/// nothing at all for the three Verkstead words itself, which no Process
/// declares.
const STORED: Record<Lifecycle, string | null> = {
  Draft: null,
  Grilling: "grilling",
  Implementing: "implementing",
  Wrapping: "wrapping",
  FollowUp: "follow-up",
  Investigating: "investigating",
  Declared: null,
  Done: null,
  Closed: null,
};

/// What one of Verkstead's own states is drawn as, read the way the page reads
/// it: off Develop's manifest for the states it declares, and Verkstead's own
/// word for the three nothing returns to.
export function stateOf(state: Lifecycle): string {
  return stateWord(whose("develop"), state, STORED[state]);
}

/// The id a Conversation in `state` carries beside it, as the server sends the
/// two together — a Conversation's view and a Timeline move alike.
///
/// Derived here so that a test moving a Conversation's state moves the id with
/// it: the word is read off the id, so a case that overrode one and not the
/// other would be drawing a state nothing named.
export function declaredOn(state: Lifecycle): {
  declared_state: string | null;
} {
  return { declared_state: STORED[state] };
}

/// And what a Timeline move or a past steer carries beside its state: the id
/// its Process gives that state, which is what the line's word is read off —
/// and nothing at all for the three Verkstead words itself.
export function declaredAt(state: Lifecycle): { declared?: string } {
  const stored = STORED[state];

  return stored === null ? {} : { declared: stored };
}

/// And the same for a sidebar row, which names the plugin providing its
/// Process — Develop's, every fixture here being a Conversation on the ladder.
export function declaredRow(state: Lifecycle): {
  declared: DeclaredState | null;
} {
  const stored = STORED[state];

  return {
    declared:
      stored === null ? null : { process: whose("develop"), state: stored },
  };
}

/// The five with only the ones `offered` names offered, for a test about a
/// plugin's tick — the rest are listed and not offered, which is what the
/// server says about a Process whose plugin is switched off.
export function offeringOnly(offered: string[]): ProcessEntry[] {
  return PROCESSES.map((entry) => ({
    ...entry,
    offered: offered.includes(entry.id),
  }));
}
