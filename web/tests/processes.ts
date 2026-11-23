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

/// What a state is drawn as, read the way the page reads it: off Develop's
/// manifest for the states it declares, and Verkstead's own word for the three
/// nothing returns to.
///
/// `state` is the word the `conversations.state` column holds, which is every
/// state there is now that a state *is* a word — `grilling`, `wrapping`,
/// `done` (Set 1765, Q29).
export function stateOf(state: string): string {
  return stateWord(whose("develop"), state);
}

/// The Process a sidebar row reads its state word against — Develop's, every
/// fixture here being a Conversation on the ladder.
///
/// A row carries the Process rather than the word, having no Process field of
/// its own, so a test factory spreads this beside the state it sets.
export function rowProcess(): { process: ProvidedProcess } {
  return { process: whose("develop") };
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
