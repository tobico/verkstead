//! What the one **Agent** control reads while it is closed: the Implementation
//! Pairing, and how many of the other roles are on something else.
//!
//! The control behind it is a panel of the role pickers — see [`AgentOptions`]
//! in [`./Setup.tsx`](./Setup.tsx) — and a panel's trigger has to say what is
//! inside it without opening: one line, a quarter of the composer box wide,
//! standing where the role pickers stood. So it says the one role the work is
//! really run under, and counts the rest.
//!
//! Which is why there is nothing here for the control's other shape: where the
//! table says the Process is run under one role there is no panel and no
//! trigger over it, and a picker showing its own choice needs nobody to compose
//! a reading of it.
//!
//! **The Repo trigger's own convention**, which is the reason the counting reads
//! this way rather than some other: that trigger says the repository's name and
//! ` +1`, ` +2` for the repos the work runs alongside, because the row is one
//! line and the names are inside the panel. This says the Implementation
//! Pairing and ` +1` for each other role picked onto a different one. A role
//! picked onto the same Pairing adds nothing, there being nothing to say about
//! it, and neither does a role picked away — *No review* is an answer rather
//! than another account.
//!
//! And **[`NOTHING`] while a role the Process uses is empty**, whichever role it
//! is: the trigger stands over a press that will refuse on exactly that, so a
//! trigger reading an account while the panel holds an unanswered picker would
//! be the row saying the work could start.
//!
//! Composed here rather than in the component for the composers' own reason —
//! two pages ask this question, of two different things. A Conversation's picks
//! are the record's, and the compose page's are what the pickers on a draft
//! nobody has created are showing. Both arrive as what each picker would send,
//! which is the one shape the two pages share.

import type { AgentType } from "../agents";
import type { Process, ProfileEntry } from "../api/types";
import * as pairing from "../pairing";
import { NOTHING } from "../picking";
import { ROLES, type Role } from "./processes";

/// What each role's picker is showing, as that picker's own `chosen` writes it:
/// a Pairing, [`pairing.NONE`] for a role picked away, and the empty string for
/// nothing chosen.
///
/// Partial because which roles are asked about at all is the Process's — a
/// Process using one role has one picker, and what is not drawn is not picked.
/// A role the Process uses and this does not carry is nothing chosen, which is
/// what an absent picker would have shown.
export type Picked = Partial<Record<Role, string>>;

/// And what the trigger reads, in the parts it is drawn out of: the mark of the
/// harness beside the words, as every reading of who runs a session is drawn —
/// see [`HarnessMark`](../HarnessMark.tsx).
export type Reading = {
  /// The Implementation Pairing's short reading, or [`NOTHING`].
  words: string;
  /// Whose mark goes in front of them, and `null` where there is no Pairing to
  /// be reading — the same `null` the row that runs nothing draws.
  mark: AgentType | null;
  /// How many other roles the Process uses are on a different Pairing. Zero
  /// draws nothing at all rather than ` +0`.
  also: number;
};

/// The reading itself, off the Process's roles and what each of their pickers is
/// showing.
///
/// `saved` is the Profiles as they stand, for the two things it decides: which
/// Pairing a pick *is* — a pick whose Profile has been removed is no longer one
/// of the rows, exactly as the picker on it falls to its placeholder — and
/// whether the Profile's name is said in the reading at all. See
/// [`pairing.shown`](../pairing.ts).
export function reading(
  process: Process,
  picked: Picked,
  saved: ProfileEntry[],
): Reading {
  const rows = pairing.pairings(saved);

  /// Which Pairing a role is on: `null` for a role picked away, and `undefined`
  /// for a role with nothing chosen — a pick that is no longer one of the rows
  /// among them, that being what the picker itself is showing nothing for.
  const on = (role: Role): pairing.Pairing | null | undefined => {
    const choice = picked[role] ?? "";

    return choice === pairing.NONE
      ? null
      : rows.find((row) => pairing.value(row) === choice);
  };

  const used = ROLES[process].uses;
  /// The role the trigger reads. Every Process uses it — the table's `uses`
  /// names it in all five rows, there being no work without something building
  /// it — so the one case this misses is a table that stopped saying so, which
  /// reads as nothing chosen rather than as some other role's account.
  const building = on("implementation");

  if (!building || used.some((role) => on(role) === undefined)) {
    return { words: NOTHING, mark: null, also: 0 };
  }

  return {
    words: pairing.shown(building, saved),
    mark: building.profile.account.agent_type,
    // Compared as picks rather than as Pairings: two roles on one Pairing sent
    // the same string, which is the whole of what *the same account and model*
    // means here.
    also: used.filter(
      (role) =>
        role !== "implementation" &&
        on(role) !== null &&
        picked[role] !== picked.implementation,
    ).length,
  };
}
