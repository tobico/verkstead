//! The Processes this server holds, as it says them: each one's name, its
//! description, whether the picker offers it, the words of the states it
//! declares, the roles it is run under, and whether it takes a Target.
//!
//! **Read off the server rather than kept here** (ADR-0030, *Process*;
//! Set 1765, Q29): every Process is data in a plugin's manifest — the five
//! Verkstead ships are bundled plugins of their own — so which ones there are
//! and what they are called is the server's to say. This module is where the
//! page asks, and there is no second list anywhere in the viewer: a row a
//! Process is drawn as, a word a state is said in and the sentence under a
//! picker row all come from here.
//!
//! **Held in one place for the whole page**, read by [`useProvidedProcesses`]
//! at the shell and again when the installed Plugins move — the harness list's
//! bargain, see `harnesses.ts` — so the plain functions that word a
//! Conversation can ask it without a query of their own.
//!
//! **A Process the list does not hold is named by what it names**, and a state
//! of one is said as its own id: a Conversation on a plugin removed under it
//! reads the word the record holds rather than a blank. Which Remove being
//! refused while anything stands on the plugin should make rare.

import { createRenderEffect, createSignal } from "solid-js";

import { listProcesses } from "../api/client";
import type {
  DeclaredState,
  Lifecycle,
  ProcessChoice,
  ProcessEntry,
  ProvidedProcess,
  RoleEntry,
} from "../api/types";
import { useReading } from "../freshness";

const [listed, setListed] = createSignal<ProcessEntry[]>([]);

/// What Verkstead calls the three states that are its own, which no Process
/// declares and none returns to (ADR-0030, *Process*).
///
/// The one state table left in the viewer, and it is Verkstead's rather than
/// any Process's: a Draft is drafting whatever it was going to be for, and
/// Done and Closed are where work stops. Every other state's word is its
/// Process's — see [`stateWord`].
const VERKSTEAD: Partial<Record<Lifecycle, string>> = {
  Draft: "Draft",
  Done: "Done",
  Closed: "Closed",
};

/// The states the work has ended in: the ladder's last rung, and the way off
/// the ladder. Neither has anything running or anything to come, which is what
/// makes them the two the status button says on their own — no status word
/// beside the state, because the word for where a finished Conversation got to
/// *is* the state, and saying it twice with a colour between would be the
/// button repeating itself.
export const ENDED: ReadonlySet<Lifecycle> = new Set<Lifecycle>([
  "Done",
  "Closed",
]);

/// What the state a Conversation stands in is called on the page.
///
/// `declared` is the id its Process gives that state, which the server sends
/// for every state but Verkstead's own three, and the word is what that
/// Process's manifest declares for it — so a Grilling reads *Grilling* off
/// Develop, and a state a plugin named reads whatever that plugin called it.
///
/// The id itself where this page holds no Process to read it against, which is
/// legible where a blank would not be, and Verkstead's own word for the three
/// that are Verkstead's.
export function stateWord(
  process: ProvidedProcess,
  state: Lifecycle,
  declared: string | null | undefined,
): string {
  return declared
    ? wordIn(processEntry(process), declared)
    : (VERKSTEAD[state] ?? "In progress");
}

/// And the same for a sidebar row, which carries the plugin providing its
/// Process rather than the word the record holds — see `DeclaredState`, which
/// is what the server sends it as.
export function rowWord(
  declared: DeclaredState | null,
  state: Lifecycle,
): string {
  return declared === null
    ? (VERKSTEAD[state] ?? "In progress")
    : wordIn(processEntry(declared.process), declared.state);
}

/// The word a Process says the state `state` in, or the state's own id where
/// this page holds no such Process.
function wordIn(entry: ProcessEntry | undefined, state: string): string {
  return entry?.states.find((each) => each.id === state)?.word ?? state;
}

/// Every Process the picker offers as a row of its own, in the order the
/// server lists them: Verkstead's own five first, in the order their rows are
/// drawn, then each installed plugin's in the order they were installed.
///
/// Empty is a page that has not read the list yet, and a server with every
/// tick off — see `defaultChoice`, which is what the composer stands on
/// either way.
export function offeredProcesses(): ProcessChoice[] {
  return listed()
    .filter((entry) => entry.offered)
    .map(choiceOf);
}

/// The choice a picker row holds for a listed Process: the plugin providing it
/// and the Process's id there, which is what every Process is named by.
function choiceOf(entry: ProcessEntry): ProcessChoice {
  return { process: { plugin: entry.plugin, id: entry.id } };
}

/// The row a picker nobody has touched stands on: the first Process the server
/// offers, which is **Develop** while its tick stands.
///
/// The server prefills a new Draft the same way, off the same order — see
/// `default_process` in `crates/server/src/conversations.rs` — so the control
/// and the record say one thing.
///
/// `null` where this page has nothing to stand on: a list it has not read yet,
/// which the first read answers a moment later, and a server with every tick
/// off and no plugin providing one. The picker then draws no row rather than
/// naming a Process nobody offered, and the press says it is not offered —
/// which is the truth, the record reading Develop with no row of its own.
export function defaultChoice(): ProcessChoice | null {
  return offeredProcesses()[0] ?? null;
}

/// The entry for the Process `process`: the plugin providing it and its id
/// there, which is how the server lists every one of them.
///
/// Offered or not, so a Conversation on a switched-off plugin's Process is
/// still worded — and `undefined` for a Process this page holds nothing for,
/// which is a list it has not read yet or a plugin removed under a
/// Conversation.
export function processEntry(
  process: ProvidedProcess,
): ProcessEntry | undefined {
  return listed().find(
    (entry) => entry.plugin === process.plugin && entry.id === process.id,
  );
}

/// And the same for a Process the composer is *picking* rather than one a
/// Conversation is on, `null` being a picker standing on nothing at all.
export function entryFor(
  choice: ProcessChoice | null,
): ProcessEntry | undefined {
  return choice === null ? undefined : processEntry(choice.process);
}

/// The roles the Process a composer is picking declares, in the order its
/// manifest writes them: what the **Agent** control draws a picker for, and
/// how many of them says which shape that control takes (Set 1765, Q26).
///
/// Empty for a Process this page holds no entry for, which draws the one
/// picker the dropdown shape is — see `control` in `roles.ts`. A Conversation
/// past drafting reads its roles off its own Pairings instead, which carry
/// the label and the mark beside the pick.
export function rolesFor(choice: ProcessChoice | null): RoleEntry[] {
  return entryFor(choice)?.roles ?? [];
}

/// Whether a Process is pointed at work that is already somewhere else, and so
/// draws the **Target** field in the Repo panel: its manifest's `target`, which
/// the server sends with the Process.
///
/// Which is also what the record's readiness waits on — see `takes_a_target` in
/// `crates/server/src/conversations.rs` — so the field and the press ask one
/// question. A Process this page holds nothing for draws no field: there is
/// nothing saying it wants one.
export function targeted(process: ProvidedProcess): boolean {
  return processEntry(process)?.target ?? false;
}

/// And the same for a Process being picked.
export function targetedFor(choice: ProcessChoice | null): boolean {
  return entryFor(choice)?.target ?? false;
}

/// Whether a Process's wrap-up is narrowed to what GitHub refuses a merge for:
/// the review and the comments settled, so nothing reads the branch, nothing
/// said on the pull request is answered, and what is left to wait on is the
/// checks and whether the pull request merges.
///
/// The question the Steer form asks before it says what wrapping up will do,
/// under Verkstead's own **Wrapping** row. Its manifest's again — its
/// `wrapping` state turns the Wrap-up's `review` and `comments` off, which is
/// the same reading the wrap-up itself makes.
export function narrowed(process: ProvidedProcess): boolean {
  return processEntry(process)?.narrowed ?? false;
}

/// Which Process a Brief that is one pull request URL and nothing else fills
/// the picker with, where this page holds it: **Fix merge issues**, that being
/// what the record is filled with.
///
/// The server's rule drawn before the save lands — see `save_brief` in
/// `crates/server/src/conversations.rs`, which writes the same one — so the
/// picker says what the record is about to. The list's own fact rather than a
/// word kept here, and `null` for a server whose Fix merge issues is unticked
/// or gone: the picker then stays where it was, as it does for a Brief naming
/// nothing.
export function filledFromAPullRequest(): ProcessChoice | null {
  const entry = listed().find((listed) => listed.filled_from_a_pull_request);

  return entry === undefined ? null : choiceOf(entry);
}

/// What a Conversation's Process is called: the name its manifest gives it, or
/// its own id where this server holds no such Process — a plugin removed under
/// a Conversation, which is legible where a blank would not be.
export function processName(process: ProvidedProcess): string {
  return processEntry(process)?.name ?? process.id;
}

/// And the sentence under a picker row's name: what kind of work it is, said
/// before the human picks it rather than found out after. Its manifest's, and
/// empty for a Process this page holds nothing for — a row offered always has
/// one, every offered row being one the server listed.
export function processDescription(choice: ProcessChoice | null): string {
  return entryFor(choice)?.description ?? "";
}

/// The value a picker's row holds for a choice, which is what tells the rows
/// apart: the plugin's uuid and the Process's id there. The empty string for a
/// picker standing on nothing, which is what [`Choosing.chosen`] reads as
/// nothing chosen.
export function choiceValue(choice: ProcessChoice | null): string {
  return choice === null ? "" : `${choice.process.plugin}/${choice.process.id}`;
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
