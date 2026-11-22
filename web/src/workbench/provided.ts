//! The Processes the plugins loaded on this server provide, as it says them:
//! each one's name, description and the words of the states it declares.
//!
//! **Read off the server rather than kept here** (ADR-0030, *Process*): a
//! plugin's Process is data in its manifest, so which ones there are and what
//! they are called is the server's to say. The five Verkstead runs itself are
//! still worded by `processes.ts` and `states.ts`; this is everything beside
//! them.
//!
//! **Held in one place for the whole page**, read by [`useProvidedProcesses`]
//! at the shell and again when the installed Plugins move — the harness list's
//! bargain, see `harnesses.ts` — so the plain functions that word a
//! Conversation can ask it without a query of their own.
//!
//! **An id the list does not hold reads as the id**, which is legible where a
//! blank would not be: a Conversation on a plugin removed under it is named by
//! what it names.

import { createRenderEffect, createSignal } from "solid-js";

import { listProcesses } from "../api/client";
import type {
  Process,
  ProcessChoice,
  ProcessEntry,
  ProvidedProcess,
} from "../api/types";
import { useReading } from "../freshness";
import { OFFERED, PROCESS } from "./processes";

const [listed, setListed] = createSignal<ProcessEntry[]>([]);

/// Every plugin's Process the picker offers as a row of its own, in the order
/// they were installed — not Verkstead's own written as data, which the picker
/// draws as the five rows it always has.
export function offeredProcesses(): ProcessEntry[] {
  return listed().filter((entry) => entry.offered && entry.built_in === null);
}

/// Whether one of Verkstead's own Processes is offered: its plugin's tick
/// stands. All five are plugins of their own, so the server lists one for each
/// of them; a list this page has not read yet offers them all, as it must
/// before the first read answers.
export function builtInOffered(process: Process): boolean {
  return (
    listed().find((entry) => entry.built_in === process)?.offered ?? true
  );
}

/// The row a picker nobody has touched stands on: **Develop** while its tick
/// stands, and otherwise the first Process still offered — Verkstead's own in
/// the order [`OFFERED`] draws them, then each plugin's in the order they were
/// installed.
///
/// The server prefills a new Draft the same way, off the same order — see
/// `default_process` in `crates/server/src/conversations.rs` — so the control
/// and the record say one thing. Develop while nothing at all is offered,
/// which is every tick off: the record reads Develop with no row of its own,
/// and Start says it is not offered rather than the page naming something
/// nobody offered.
export function defaultChoice(): ProcessChoice {
  const built = OFFERED.find(builtInOffered);

  if (built !== undefined) {
    return { process: built };
  }

  const [first] = offeredProcesses();

  return first === undefined
    ? { process: "Develop" }
    : { process: "Provided", provided: { plugin: first.plugin, id: first.id } };
}

/// The plugin's Process `provided` names, where this server holds it —
/// offered or not, so a Conversation on a switched-off plugin's is still
/// worded.
export function providedProcess(
  provided: ProvidedProcess | null,
): ProcessEntry | undefined {
  if (provided === null) {
    return undefined;
  }
  return listed().find(
    (entry) => entry.plugin === provided.plugin && entry.id === provided.id,
  );
}

/// What a Conversation's Process is called: one of the five's own name, or a
/// plugin's as its manifest says it, or its id where this server holds none.
export function processName(
  process: Process,
  provided: ProvidedProcess | null,
): string {
  if (process !== "Provided" || provided === null) {
    return PROCESS[process];
  }
  return providedProcess(provided)?.name ?? provided.id;
}

/// The word a declared state is drawn in: the one its Process's manifest
/// gives it, or its id.
export function declaredWord(
  provided: ProvidedProcess | null,
  state: string | null,
): string {
  if (state === null) {
    return "";
  }
  return (
    providedProcess(provided)?.states.find((each) => each.id === state)
      ?.word ?? state
  );
}

/// The value a picker's row holds for a choice, which is what tells the
/// rows apart: a Process's own word, or a plugin's uuid and the Process's id.
export function choiceValue(choice: ProcessChoice): string {
  return choice.process === "Provided" && choice.provided !== undefined
    ? `${choice.provided.plugin}/${choice.provided.id}`
    : choice.process;
}

/// Say what the Processes are — what the shell's read does.
export function knowProvidedProcesses(entries: ProcessEntry[]): void {
  setListed(entries);
}

/// Read the list off the server and say it here, for as long as the caller
/// is mounted, and again whenever the installed Plugins move.
export function useProvidedProcesses(): void {
  const reading = useReading(() => ({
    queryKey: ["processes"],
    queryFn: listProcesses,
    freshness: { reconcile: "id" },
  }));

  createRenderEffect(() => {
    const entries = reading.data;

    if (entries !== undefined) {
      setListed(entries);
    }
  });
}
