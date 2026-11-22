//! The words a Conversation's Processes are said in, which of them the picker
//! offers, and what is drawn around the roles each one is run under.
//!
//! **Which roles those are is no longer here**: a Process declares them in its
//! manifest, name and label, one of them the building role, and the server
//! sends them with the Process (ADR-0030, *Process*) — see `provided.ts`, which
//! is where a page asks. What is left here is the words and the shapes drawn
//! around them: the row a picker offers that is no account at all, how many
//! roles a Start says it is waiting on, and which shape the one **Agent**
//! control takes.
//!
//! The wire carries a Process as a name rather than a sentence — the record's
//! own word for what kind of work this is — and what the human reads is the
//! page's to choose. For four of the five the two are the same word, and this
//! exists for the one where they are not: *Fix merge issues* is how somebody
//! would say it, and `FixMergeIssues` is only how a variant is spelled.
//!
//! In one place because the same Process is written in two: the picker on the
//! composer, and the Configuration under a frozen Brief. The thing picked and
//! the thing read back cannot be allowed to come to be called different things —
//! which is `states.ts`'s reason, and `directions.ts`'s.

import type {
  PickedView,
  Process,
  RoleEntry,
  RolePairingView,
} from "../api/types";

/// The five Verkstead runs itself, which this module words. A plugin's is
/// worded by its manifest — see `provided.ts`.
export type BuiltIn = Exclude<Process, "Provided">;

/// What each Process is called on the page.
///
/// All five, which is every one the picker offers and every one the wire
/// carries: a record naming a Process this viewer refused to word would be a
/// pane with a hole in it.
export const PROCESS: Record<Process, string> = {
  Develop: "Develop",
  Investigate: "Investigate",
  Review: "Review",
  Tinker: "Tinker",
  FixMergeIssues: "Fix merge issues",
  // A plugin's, whose name is its manifest's — see `processName` in
  // `provided.ts`, which every place naming a Conversation's Process asks.
  Provided: "Plugin process",
};

/// And the sentence under each one's name in the picker's dropped rows: what
/// kind of work it is, said before the human picks it rather than found out
/// after.
///
/// The human's own words, kept as they wrote them — *PR* included — because a
/// sentence about which Process to pick is theirs to phrase. Drawn in the rows
/// and nowhere else: the closed control and a frozen Brief say which one was
/// picked, and somebody reading either has already chosen.
///
/// All five, for [`PROCESS`]'s reason: a row offered with no sentence under it
/// would be the one row of the list that left the human guessing.
///
/// Not a plugin's, whose sentence is its manifest's — see `provided.ts`.
export const DESCRIPTION: Record<BuiltIn, string> = {
  Develop:
    "Work on something complex, starting with a grilling session and proceeding through a step-by-step development process",
  Investigate: "Ask questions about a code base without committing any code",
  Review: "Review an existing PR or branch, and fix any issues found",
  Tinker: "Make a series of simple one-off changes with minimal fuss",
  FixMergeIssues: "Fix up a PR which is unable to merge",
};

/// And which of them the picker offers, in the order it draws them.
///
/// **The viewer's list, and the server keeps the other** — `LANDED` in
/// `crates/server/src/conversations.rs`, which is what the record will accept.
/// Two lists rather than one because they are two different statements: that one
/// refuses a Process whose stage has not landed, and this one is what the human
/// is offered in the first place. A stage that brings a Process to life adds to
/// both, and each of them is written knowing the other is there.
///
/// All five, Fix Merge Issues having landed last. A Process is offered only once
/// its stage has landed, as an agent type is offered only once it can launch the
/// real thing — and *Fix merge issues* sits under the Review it is the narrowed
/// form of, the order being the order the rows are drawn in.
export const OFFERED: BuiltIn[] = [
  "Develop",
  "Tinker",
  "Investigate",
  "Review",
  "FixMergeIssues",
];

/// And which of them are pointed at work that is already somewhere else, and
/// so draw the **Target** field in the Repo panel.
///
/// **The viewer's list, and the server keeps the other** — `takes_a_target` in
/// `crates/server/src/conversations.rs`, which is what the record's readiness
/// waits on. Two lists for `OFFERED`'s reason: one says what the panel draws
/// and the other says what Start waits for, and a stage that gives a Process a
/// target adds to both.
///
/// Beside the role table because it is the same kind of fact about a Process,
/// written in the same place: **Fix Merge Issues** added itself here when its
/// stage landed, and the field appeared under its Branch field without a line
/// changing in either composer.
export const TARGETED: Process[] = ["Review", "FixMergeIssues"];

/// Whether this Process is pointed at a target — the question each composer
/// asks before drawing the field, as [`uses`] is the one it asks before drawing
/// a role's picker.
export function targeted(process: Process): boolean {
  return TARGETED.includes(process);
}

/// And which of them wrap up narrowed to what GitHub refuses a merge for: the
/// review and the comments settled, so nothing reads the branch, nothing said on
/// the pull request is answered, and what is left to wait on is the checks and
/// whether the pull request merges.
///
/// **The viewer's list, and the server keeps the other** — `narrows_the_wrap_up`
/// in `crates/server/src/conversations.rs`, which the take-up and the two
/// watchers read. Two lists for `OFFERED`'s reason: that one decides what runs,
/// and this one decides what the human is told is about to.
///
/// **Fix Merge Issues**, and nothing else — every other Process reads the branch
/// and answers what is said on the pull request.
export const NARROWED: Process[] = ["FixMergeIssues"];

/// Whether this Process's wrap-up is a narrowed one — the question the Steer form
/// asks before it says what wrapping up will do.
export function narrowed(process: Process): boolean {
  return NARROWED.includes(process);
}

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
/// holding a roadmap has its brief answered for it, and one holding a pull
/// request off the retired menu has its target answered the same way. So the
/// caller says which clauses it is asking for, and the wording is here.
export function needed(
  process: Process,
  count: number,
  asked: { brief: boolean; target: boolean },
): string {
  const wanted = [
    ...(asked.brief ? ["a brief"] : []),
    ...(asked.target && targeted(process) ? ["a target"] : []),
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
