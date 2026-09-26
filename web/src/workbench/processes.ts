//! The words a Conversation's Processes are said in, which of them the picker
//! offers, and which roles each one is run under — down to the rows each of
//! those roles' pickers offers above the Pairings.
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

import type { Process } from "../api/types";

/// What each Process is called on the page.
///
/// All five, including the one nothing offers yet: the wire carries every one
/// of them, so a record naming one this viewer refused to word would be a pane
/// with a hole in it.
export const PROCESS: Record<Process, string> = {
  Develop: "Develop",
  Investigate: "Investigate",
  Review: "Review",
  Tinker: "Tinker",
  FixMergeIssues: "Fix merge issues",
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
/// Four rows for now. A Process is offered only once its stage has landed, as an
/// agent type is offered only once it can launch the real thing.
export const OFFERED: Process[] = [
  "Develop",
  "Tinker",
  "Investigate",
  "Review",
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
/// written in the same place: **Fix Merge Issues** adds itself here when its
/// stage lands, and the field appears under its Branch field without a line
/// changing in either composer.
export const TARGETED: Process[] = ["Review"];

/// Whether this Process is pointed at a target — the question each composer
/// asks before drawing the field, as [`uses`] is the one it asks before drawing
/// a role's picker.
export function targeted(process: Process): boolean {
  return TARGETED.includes(process);
}

/// One of the roles a Conversation's sessions are run under, spelled the way
/// the record's own fields spell it — `grilling_pairing`, and the two beside
/// it.
export type Role = "grilling" | "implementation" | "review";

/// And what a Process says about who runs it.
export type Roles = {
  /// The roles it uses, in the order a control draws them. A Process draws a
  /// picker per role here and no others, and the press waits on every one of
  /// them.
  uses: Role[];
  /// Those of them that may be picked away — the picker's row that is no
  /// account at all. A role not named here has to be answered with one.
  ///
  /// **What the picker draws**, which is what makes it worth reading rather
  /// than a claim nothing checks: a row comes out of the app by coming out of
  /// this table, so the row and the control cannot say different things.
  away: Role[];
  /// And which shape the one **Agent** control takes: a panel where there are
  /// several roles to stack under their labels, and the flat Pairing dropdown
  /// where there is one.
  control: "panel" | "dropdown";
};

/// Which roles each Process uses, which of them may be picked away, and which
/// shape its control takes.
///
/// [ADR-0020](../../../docs/adr/0020-a-conversation-has-a-process.md) carries
/// this table, and this is that table written down where the web reads it: the
/// two composers draw off it and the start's own waiting text counts off it, so
/// a Process arriving later is a row added here rather than a branch added in
/// each of those places.
///
/// All five, for [`PROCESS`]'s reason — the wire carries every one of them, and
/// a record naming a Process this viewer had no roles for would be a composer
/// with nothing in its setup row.
export const ROLES: Record<Process, Roles> = {
  Develop: {
    // One row that is no account, the review's: *No grilling* is retired, a
    // Brief that wants no interview being a Tinker rather than a hole in this
    // Process.
    uses: ["grilling", "implementation", "review"],
    away: ["review"],
    control: "panel",
  },
  Investigate: {
    uses: ["implementation"],
    away: [],
    control: "dropdown",
  },
  Review: {
    // No row that is no account: a Review without a review is Fix Merge Issues
    // with the comments answered, so the review here is answered with a Pairing
    // or the press waits. The one Process of the four that offers none.
    uses: ["implementation", "review"],
    away: [],
    control: "panel",
  },
  Tinker: {
    uses: ["implementation", "review"],
    away: ["review"],
    control: "panel",
  },
  FixMergeIssues: {
    uses: ["implementation"],
    away: [],
    control: "dropdown",
  },
};

/// Whether a Process uses a role — the question each composer asks of each
/// picker it might draw.
export function uses(process: Process, role: Role): boolean {
  return ROLES[process].uses.includes(role);
}

/// The words a role's picker says the row that runs no session in, where it
/// offers one: *No review* above the review Pairings, and nowhere else.
///
/// No entry for the implementation role, which has no such row anywhere —
/// there is no work without something building it, which is why every
/// Process's `uses` names it and no Process's `away` can. And none for the
/// grilling role any more: *No grilling* is retired, and what it was for is the
/// Tinker Process.
const AWAY: Partial<Record<Role, string>> = {
  review: "No review",
};

/// And the row this Process's picker for this role offers, or `undefined`
/// where the role has to be answered with an account.
///
/// The second question each composer asks of each picker it draws, after
/// [`uses`]: which rows it offers is the table's as much as whether it is
/// drawn, so the two cannot come apart — a row a composer wrote in by hand
/// would go on being offered through the stage that retired it.
export function away(process: Process, role: Role): string | undefined {
  return ROLES[process].away.includes(role) ? AWAY[role] : undefined;
}

/// How an inert Start names the roles it is waiting on, in the middle of the
/// sentence it carries in its `title`: *every role*, *both roles* or *one
/// role*.
///
/// Counted off the table rather than fixed at three, which is the whole of what
/// this changes about the words: a Review waits on two and says *both roles*,
/// exactly as the sentence written for a held pull request always did, and a
/// Process with one role says so.
export function roles(process: Process): string {
  const count = ROLES[process].uses.length;
  return count === 1 ? "one role" : count === 2 ? "both roles" : "every role";
}

/// And the whole of what an inert Start is waiting on, as the middle of that
/// sentence: a brief, a target, and the roles counted off the table.
///
/// In one place because both composers say it — a draft's own press and the
/// compose page's — and each of them asks for a different subset: a page
/// holding a roadmap has its brief answered for it, and one holding a pull
/// request off the retired menu has its target answered the same way. So the
/// caller says which clauses it is asking for, and the wording is here.
export function needed(
  process: Process,
  asked: { brief: boolean; target: boolean },
): string {
  const wanted = [
    ...(asked.brief ? ["a brief"] : []),
    ...(asked.target && targeted(process) ? ["a target"] : []),
    `${roles(process)} picked and working`,
  ];

  // The comma before the *and* is what the sentence has always had, and it is
  // what keeps a three-part list readable: *a brief, a target, and both roles*.
  const last = wanted.pop()!;
  return wanted.length === 0 ? last : `${wanted.join(", ")}, and ${last}`;
}

/// What each role's picker is called, which is the role's own name: the tests,
/// the Brief's setup facts and the Steer form all speak Grilling,
/// Implementation and Review, and a control that renamed them would be the one
/// place they are called something else.
export const ROLE: Record<Role, string> = {
  grilling: "Grilling",
  implementation: "Implementation",
  review: "Review",
};

/// And what one of them is labelled under a Process, which is the table's third
/// column asked of a picker.
///
/// Inside the panel it is the role's own name, there being several of them
/// stacked and a name apiece being the whole of what tells them apart. Where the
/// table says a dropdown there is no panel and no trigger over it: the picker
/// *is* the one **Agent** control, standing in the row where the trigger would
/// have stood, so the row's own label is what names it.
export function label(process: Process, role: Role): string {
  return ROLES[process].control === "dropdown" ? "Agent" : ROLE[role];
}
