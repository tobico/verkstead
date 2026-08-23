//! Profile-and-model pairings: the rows every picker of "who runs this" offers,
//! and the one string a pairing travels as inside a `<select>`.
//!
//! A profile says which account, and its list says what that account can launch.
//! Neither half alone starts a session, so what is picked is both at once: one
//! flat row per profile-and-model combination, `profile — model`, one tap to
//! choose. A two-stage profile-then-model picker was considered and rejected —
//! it scales better and costs a tap every time, and the counts stay small.
//!
//! There is no default model anywhere, which is why nothing here invents one: a
//! profile with no model beside it is not a pairing, and the pickers draw it as
//! nothing chosen.

import type { PairingView, ProfileEntry, ProfileChoice } from "./api/types";

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

/// And what the human reads.
export function label(pairing: Pairing): string {
  return `${pairing.profile.name} — ${pairing.model}`;
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
