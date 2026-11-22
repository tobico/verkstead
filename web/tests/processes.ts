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

import type {
  DeclaredState,
  Lifecycle,
  Process,
  ProcessEntry,
} from "../src/api/types";
import { stateWord } from "../src/workbench/provided";
import listed from "./fixtures/processes.json" with { type: "json" };

/// The five, in the order the server lists them and so the order the picker
/// draws their rows.
export const PROCESSES = listed as ProcessEntry[];

/// Which of them the picker offers, by the word the record holds — the five,
/// each while its plugin's tick stands, which is how a server with nothing
/// unticked lists them.
export const OFFERED: Process[] = PROCESSES.map((entry) => entry.built_in!);

/// The entry for one of them, by the word the record holds.
export function processEntry(process: Process): ProcessEntry {
  const entry = PROCESSES.find((listed) => listed.built_in === process);

  if (entry === undefined) {
    throw new Error(`no built-in Process ${process} is listed`);
  }

  return entry;
}

/// What it is called: the name its manifest gives it, which is the row's label
/// and the word a frozen Brief reads back.
export function named(process: Process): string {
  return processEntry(process).name;
}

/// And the sentence under that row.
export function described(process: Process): string {
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
  return stateWord("Develop", null, state, STORED[state]);
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
  const develop = processEntry("Develop");

  return {
    declared:
      stored === null
        ? null
        : {
            process: { plugin: develop.plugin, id: develop.id },
            state: stored,
          },
  };
}

/// The five with only the ones `offered` names offered, for a test about a
/// plugin's tick — the rest are listed and not offered, which is what the
/// server says about a Process whose plugin is switched off.
export function offeringOnly(offered: Process[]): ProcessEntry[] {
  return PROCESSES.map((entry) => ({
    ...entry,
    offered: offered.includes(entry.built_in!),
  }));
}
