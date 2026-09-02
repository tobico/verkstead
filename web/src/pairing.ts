//! Profile-and-model pairings: the rows every picker of "who runs this" offers,
//! and the one string a pairing travels as inside a `<select>`.
//!
//! A profile says which account, and its list says what that account can launch.
//! Neither half alone starts a session, so what is picked is both at once: one
//! flat row per profile-and-model combination, "Claude Code Fable 5 — Work", one
//! tap to choose. A two-stage profile-then-model picker was considered and
//! rejected — it scales better and costs a tap every time, and the counts stay
//! small.
//!
//! How a row reads is not this module's own: it is the reading every site that
//! says who runs a session shares, composed in [`./agents.ts`](./agents.ts) out
//! of the backend, the model and the profile's name. What is here is the pairing
//! vocabulary — the rows, and the string one travels as.
//!
//! There is no default model anywhere, which is why nothing here invents one: a
//! profile with no model beside it is not a pairing, and the pickers draw it as
//! nothing chosen.

import { briefly, reading } from "./agents";
import type {
  PairingView,
  PickedView,
  ProfileEntry,
  ProfileChoice,
  RoleChoice,
} from "./api/types";

/// One row of a pairing picker: an account, and one thing that account can run.
export type Pairing = {
  profile: ProfileEntry;
  model: string;
};

/// Every pairing the saved profiles come to, in the order they are listed —
/// profiles by name, and each profile's models as the human typed them.
export function pairings(profiles: ProfileEntry[]): Pairing[] {
  return profiles.flatMap((profile) =>
    profile.models.map((model) => ({ profile, model })),
  );
}

/// What one row sends when it is the choice.
///
/// The two halves in one string because a `<select>` carries one string. The id
/// comes first and the model is whatever follows the first colon, so a model
/// with a colon in its name still arrives whole.
export function value(pairing: Pairing): string {
  return `${pairing.profile.id}:${pairing.model}`;
}

/// And what the human reads: the shared reading, made of what the pairing
/// carries.
///
/// A `PairingView` as well as a [`Pairing`], the model being the half a
/// conversation can have settled without — a profile picked before models were
/// paired beside them reads as the backend and the profile, which is the half
/// there is.
///
/// `saved` is the profiles as they stand, which decides whether the profile's
/// own name is said at all — see [`reading`](./agents.ts).
export function label(
  pairing: { profile: ProfileEntry; model: string | null },
  saved: ProfileEntry[] | undefined,
): string {
  return reading(
    {
      agent: pairing.profile.account.agent_type,
      model: pairing.model,
      profile: pairing.profile.name,
    },
    saved,
  );
}

/// And what the closed control reads, which is the same reading less the
/// backend's name — see [`briefly`](./agents.ts). One picker draws both: the
/// rows say the whole of it, and the trigger says the half that is not already
/// drawn in the mark beside it.
export function shown(
  pairing: { profile: ProfileEntry; model: string | null },
  saved: ProfileEntry[] | undefined,
): string {
  return briefly(
    {
      agent: pairing.profile.account.agent_type,
      model: pairing.model,
      profile: pairing.profile.name,
    },
    saved,
  );
}

/// What is chosen now, as [`value`] would have written it.
///
/// The empty string for nothing chosen — which includes a profile chosen before
/// pairings existed, that being half a choice and so a choice to make again.
export function chosen(pairing: PairingView | null): string {
  return pairing?.model ? `${pairing.profile.id}:${pairing.model}` : "";
}

/// One picked string, back into the two halves the server is sent.
export function choice(picked: string): ProfileChoice {
  const colon = picked.indexOf(":");

  return {
    profile_id: Number(picked.slice(0, colon)),
    model: picked.slice(colon + 1),
  };
}

/// The row that says a role runs no session at all, as it travels inside a
/// `<select>` — "No grilling" on one picker and "No review" on the other.
///
/// Not the empty string, which is the picker's own placeholder: nothing chosen
/// and *chosen to run nothing* are different states, and one of them lets the
/// work start.
///
/// A colon and nothing after it, which no pairing can spell — [`value`] writes
/// a profile id before its colon and there is no profile numbered nothing.
export const NONE = ":";

/// What is chosen now for a role that can be picked away as well as paired.
export function settled(picked: PickedView): string {
  if (picked === "Skipped") {
    return NONE;
  }

  return picked === "Nothing" ? "" : chosen(picked.Under);
}

/// And the Pairing behind it, where one was picked — for the panes that draw
/// what a Conversation settled rather than offer it.
export function under(picked: PickedView): PairingView | null {
  return picked === "Nothing" || picked === "Skipped" ? null : picked.Under;
}

/// One picked string, back into the body a picker that offers that row sends: a
/// pairing, or the row that runs nothing.
export function role(picked: string): RoleChoice {
  return { pairing: picked === NONE ? null : choice(picked) };
}
