//! What is drawn around the roles a Conversation's sessions are run under, and
//! the words Verkstead itself says about them.
//!
//! **Which roles those are is not here, and neither is anything about a
//! Process**: a Process declares its roles in its manifest, name and label,
//! one of them the building role, and the server sends them with the Process
//! (ADR-0030, *Process*) — see `provided.ts`, which is where a page asks for
//! that and for every word a Process or one of its states is said in
//! (Set 1765, Q29). What is left here is the shapes drawn around them: the row
//! a picker offers that is no account at all, how many roles a Start says it is
//! waiting on, and which shape the one **Agent** control takes.
//!
//! In one place because each of these is said in more than one: the composer's
//! panel and the Conversation's, a draft's own press and the compose page's. A
//! control labelled two ways is two controls to the person reading them.

import type { PickedView, RoleEntry, RolePairingView } from "../api/types";

/// One of the roles a Conversation's sessions are run under: the **name** its
/// Process declares it by.
///
/// A word rather than a union of three, because which roles there are is the
/// Process's to say (ADR-0030, *Process*) and a plugin's may name one nobody
/// has used before. The three Verkstead's own Processes declare are `grilling`,
/// `implementation` and `review`; they have no standing here beyond being the
/// three their manifests happen to write.
export type Role = string;

/// The row a role's picker offers that is no account at all, where its manifest
/// says it offers one: *No review*, and whatever a plugin's own role calls it.
///
/// The label lowercased, which is how the review's row has always read — and
/// `undefined` for a role that has to be answered with an account. Composed
/// here rather than carried on the wire because it is a word of Verkstead's
/// about a row Verkstead draws; what the role is *called* is the manifest's.
export function away(role: RoleEntry | RolePairingView): string | undefined {
  return role.away ? `No ${role.label.toLowerCase()}` : undefined;
}

/// What a Conversation settled for the role named `role` — nothing where its
/// Process declares no such role, which is the same answer as a picker nobody
/// has filled: the role is one no picker is drawn for.
export function settledFor(
  pairings: RolePairingView[],
  role: string,
): PickedView {
  return pairings.find((one) => one.role === role)?.picked ?? "Nothing";
}

/// How an inert Start names the roles it is waiting on, in the middle of the
/// sentence it carries in its `title`: *every role*, *both roles* or *one
/// role*.
///
/// Counted off the Process's own roles rather than fixed at three, which is the
/// whole of what this changes about the words: a Review declares two and says
/// *both roles*, exactly as the sentence written for a held pull request always
/// did, and a Process with one says so.
export function roles(count: number): string {
  return count === 1 ? "one role" : count === 2 ? "both roles" : "every role";
}

/// And the whole of what an inert Start is waiting on, as the middle of that
/// sentence: a brief, a target, and the roles counted off the Process.
///
/// In one place because both composers say it — a draft's own press and the
/// compose page's — and each of them asks for a different subset: a page
/// holding a roadmap has its brief answered for it, one holding a pull
/// request off the retired menu has its target answered the same way, and a
/// Process whose manifest says it takes no target never had one to wait on.
/// So the caller says which clauses it is asking for — `targeted` in
/// `provided.ts` being the question it asks about the target — and the
/// wording is here.
export function needed(
  count: number,
  asked: { brief: boolean; target: boolean },
): string {
  const wanted = [
    ...(asked.brief ? ["a brief"] : []),
    ...(asked.target ? ["a target"] : []),
    `${roles(count)} picked and working`,
  ];

  // The comma before the *and* is what the sentence has always had, and it is
  // what keeps a three-part list readable: *a brief, a target, and both roles*.
  const last = wanted.pop()!;
  return wanted.length === 0 ? last : `${wanted.join(", ")}, and ${last}`;
}

/// Which shape the one **Agent** control takes: a panel where there are several
/// roles to stack under their labels, and the flat Pairing dropdown where there
/// is one.
///
/// Counted off the Process's roles rather than read out of a table of its own —
/// [ADR-0020](../../../docs/adr/0020-a-conversation-has-a-process.md)'s table
/// is the manifests' now (ADR-0030, *Process*). A panel over one picker would
/// be a press to reach a press.
///
/// A Process whose plugin this page holds nothing for has no roles to draw, and
/// the dropdown is what that falls to: one picker and no panel over it.
export function control(count: number): "panel" | "dropdown" {
  return count > 1 ? "panel" : "dropdown";
}

/// And what one role's picker is labelled under a Process.
///
/// Inside the panel it is the role's own label, there being several of them
/// stacked and a label apiece being the whole of what tells them apart. Where
/// there is one role there is no panel and no trigger over it: the picker *is*
/// the one **Agent** control, standing in the row where the trigger would have
/// stood, so the row's own label is what names it.
export function label(count: number, role: RoleEntry | RolePairingView): string {
  return control(count) === "dropdown" ? "Agent" : role.label;
}
