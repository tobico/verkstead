//! What a Conversation is called while nobody has named its branch.
//!
//! A Conversation is started on a branch name Verkstead invented, because the
//! record needs one from the moment it exists and nobody has thought about the
//! work yet. A name nobody chose says nothing about the work, so while the
//! Conversation is still a draft none of it is drawn: the row and the pane
//! header call it a Draft, and the branch field stands empty under a
//! placeholder saying what leaving it empty means.
//!
//! The field is a draft's alone. The branch is a plan until the work starts and
//! a fact from the moment it is cut, so from the press there is nothing to type
//! into and nothing to leave empty — the setup card has gone, and what the work
//! branched from is reported wherever it is reported.
//!
//! The title outlasts it. Starting the work is not what makes an invented name
//! worth reading: the first session is told to switch the branch to something
//! the Brief is about, and until it has, the name says exactly what it said
//! while this was a draft. So the row and the header keep saying *Draft* through
//! that, and say the branch the moment it is over — the session renamed it, or
//! the session ended and the name it left is the one this is called by.
//!
//! In one place because the same rules are read in five: the sidebar row, what
//! it says read aloud, the pane header, the setup card's field, and the summary
//! under a frozen Brief.

import type { Lifecycle } from "../api/types";

/// The three facts either shape carries about its branch: what it is called,
/// whether anybody chose that, and where the work has got to.
///
/// Structural rather than one of the two view types, because both carry all
/// three and neither is the shape this is about.
interface Named {
  branch: string;
  branch_named: boolean;
  state: Lifecycle;
}

/// And the fourth, which only the title reads: whether the name is still the
/// first session's to replace.
interface Titled extends Named {
  naming: boolean;
}

/// What a Conversation with no name of its own is called, which is what it is.
///
/// Two drafts against one Repo reading the same is two drafts: they are few and
/// they are short-lived, and a name invented to tell them apart would be the
/// name this is here to stop drawing.
export const DRAFT = "Draft";

/// What the branch field says while it stands empty, which is what leaving it
/// empty does.
export const AUTOMATIC = "Automatically select";

/// The branch name where there is one to draw, and nothing at all where the
/// name is still Verkstead's own.
///
/// What a field prefilled with the branch holds, and what a companion mirroring
/// it comes to until there is a name to mirror.
export function chosen(conversation: Named): string {
  return conversation.branch_named || conversation.state !== "Draft"
    ? conversation.branch
    : "";
}

/// What to call a Conversation: its branch where there is one to draw, and
/// *Draft* where there is not.
///
/// One rule more than the field's. A name nobody has settled on is not drawn
/// while the Conversation is a draft, and goes on not being drawn while the
/// session that was told to replace it still might — see `naming` on either
/// view type. What ends that is the rename or the session, and both of them end
/// it for good.
export function titled(conversation: Titled): string {
  const settled =
    conversation.branch_named ||
    (conversation.state !== "Draft" && !conversation.naming);

  return settled ? conversation.branch : DRAFT;
}
