//! The words a Conversation's Processes are said in, and which of them the
//! picker offers.
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
/// All five, including the four nothing offers yet: the wire carries every one
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
/// One row for now. A Process is offered only once its stage has landed, as an
/// agent type is offered only once it can launch the real thing.
export const OFFERED: Process[] = ["Develop"];
