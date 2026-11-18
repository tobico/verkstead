//! The harnesses this server can run a session on, as it says them: each one's
//! id, name, mark, the paths an account of it is made of, and whether its login
//! is Verkstead's to run.
//!
//! **Read off the server rather than kept here** (ADR-0030, *Agent*): a harness
//! is a Plugin of the Agent Category, so which ones there are is the server's
//! to say, and a table here keyed by a closed list of words would be one that
//! could not draw a harness a plugin brought. Every site that names a harness,
//! draws its mark or asks for its paths reads it through this module.
//!
//! **Held in one place for the whole page**, read by [`useHarnesses`] at the
//! shell and again when the installed Plugins move — an Agent plugin that
//! loads adds its harness — so that the plain functions that compose a reading
//! (see `agents.ts`) can ask it without a query of their own. A share, which has no server to ask, is handed the
//! list in its bundle and says it here with [`knowHarnesses`].
//!
//! **An id the list does not hold reads as the id**, which is legible where a
//! blank would not be: a Profile written on a device with a plugin this one has
//! not got is drawn broken, and named by what it names.

import { createRenderEffect, createSignal } from "solid-js";

import { listHarnesses } from "./api/client";
import type { AgentType, HarnessEntry } from "./api/types";
import { useReading } from "./freshness";

const [listed, setListed] = createSignal<HarnessEntry[]>([]);

/// Every harness this server can run, in the order the pickers offer them.
export function harnesses(): HarnessEntry[] {
  return listed();
}

/// The harness of `id`, where this server has it.
export function harness(id: AgentType): HarnessEntry | undefined {
  return listed().find((entry) => entry.id === id);
}

/// What a harness is called wherever a human reads it: its own name, or its id
/// where this server has no harness of that id.
export function harnessName(id: AgentType): string {
  return harness(id)?.name ?? id;
}

/// Say what the harnesses are — what the shell's read does, and what a share
/// does with the list its bundle carries.
export function knowHarnesses(entries: HarnessEntry[]): void {
  setListed(entries);
}

/// Read the list off the server and say it here, for as long as the caller is
/// mounted, and again whenever the installed Plugins move.
export function useHarnesses(): void {
  const reading = useReading(() => ({
    queryKey: ["harnesses"],
    queryFn: listHarnesses,
    freshness: { reconcile: "id" },
  }));

  // Said as it lands rather than read through the query wherever a name is
  // wanted, which is what lets a plain function ask.
  createRenderEffect(() => {
    const entries = reading.data;

    if (entries !== undefined) {
      setListed(entries);
    }
  });
}
