//! The agent types a session can be run under, and the one way a session's
//! account, model and agent are read out together.
//!
//! Every place that says who runs a session used to say it its own way — the
//! pickers one way, the status button another, the record of a finished run not
//! at all. One reading serves all of them now, and this is where it is composed:
//! the backend's own name, the model's own name, and the Profile's name after an
//! em dash where the Profile is the half that tells two runs apart.
//!
//! **The reading is composed rather than kept per Pairing.** A Profile carries
//! its agent type and a model id and nothing else; every word a human reads is
//! made out of those two here, so a Profile saved a year ago reads the way one
//! saved today does, and a model the build has only just learned the name of
//! needs nothing written down beside it.
//!
//! And what goes in is three plain fields rather than a Pairing, because half
//! the sites that draw it have no Pairing to hand: what a finished run wrote down
//! is a profile name, a model id and an agent type, and it is read exactly as a
//! Pairing is.
//!
//! Two shared vocabularies feed it: the names above, and the models in
//! [`./models.ts`](./models.ts), each of which now says whose model it is.

import type { ProfileAccount, ProfileEntry } from "./api/types";
import { known, prettify } from "./models";

/// Which agent a Profile runs, which is the discriminator its account is shaped
/// by.
///
/// Read off the account rather than spelled out again, and read off it here
/// rather than in whichever module happened to need it first: what a Profile
/// runs and what a finished session ran are one fact about one set of backends,
/// and the wire's own `AgentType` — which a Timeline Event carries, having no
/// account to be shaped by — is written in the same four words on purpose.
///
/// So a fifth backend arrives in this type by being added to the record, and
/// [`AGENT_NAME`] below is what will not compile until it has been given a name
/// to read as.
export type AgentType = ProfileAccount["agent_type"];

/// What each agent type is called wherever a human reads it.
///
/// The backend's own name rather than the discriminator, which is the word the
/// record is written in and not one anybody would recognise their account by.
export const AGENT_NAME: Record<AgentType, string> = {
  Claude: "Claude Code",
  Codex: "Codex",
  Grok: "Grok Build",
  OpenCode: "OpenCode",
};

/// What one session was, or would be, run under: the three recorded facts the
/// reading is made of.
///
/// Plain fields rather than a Pairing, because half the sites that draw this
/// have no Pairing to hand — a finished run holds the profile's *name* and the
/// model *id* it was launched with, the Profile itself being a thing that can
/// since have been renamed or removed. What is written down is what is read.
export type Said = {
  /// Which backend ran it. `null` for a record from before the agent was
  /// written down, which reads as the model and the profile alone rather than
  /// as a guess.
  agent: AgentType | null;
  /// The model id it ran on. `null` — or empty — is a Profile picked before a
  /// model was paired beside it, which is half a choice and reads as one.
  model: string | null;
  /// And the name of the Profile whose account it ran as.
  profile: string;
};

/// How one of them reads: the backend, the model, and the Profile's own name
/// where that is what tells two of them apart.
///
/// "Claude Code Fable 5 — Work", "Grok 4.6", "OpenCode Minimax M2.1". The
/// model's name comes first in nobody's reading: what a human is choosing
/// between is accounts on backends, and the backend is the word that sorts
/// them.
///
/// `saved` is the Profiles as they stand, which decides one thing only — whether
/// the Profile's name is worth saying. A backend with one account needs no name
/// after its model, and a list that has not been read yet says the name, because
/// saying it is never wrong and dropping it can misattribute a run.
export function reading(said: Said, saved: ProfileEntry[] | undefined): string {
  const harness = said.agent === null ? null : AGENT_NAME[said.agent];
  const model = said.model ? prettify(said.model) : null;

  // Joined rather than appended, so that a record holding nothing but a profile
  // name reads as the name rather than as an em dash with a name after it.
  return [
    words(harness, said.model, model),
    tells(said, saved) ? said.profile : "",
  ]
    .filter((part) => part !== "")
    .join(" — ");
}

/// The reading's first half: the backend and the model, with the backend dropped
/// where the model's name has already said it.
///
/// Dropped on the model's *name* rather than on its id, which is why an id the
/// build does not know keeps the backend beside it: `claude-opus-9` under Claude
/// Code would collapse to itself, and a reading that quietly stopped naming the
/// backend on every model that shipped after the build would be the list going
/// stale in the one place it is not allowed to.
function words(
  harness: string | null,
  id: string | null,
  model: string | null,
): string {
  if (harness === null) {
    // Nothing recorded at all is not this function's to invent a word for: what
    // is left is the model, and where there is no model either the caller's own
    // fallback stands in place of the whole reading.
    return model ?? "";
  }

  if (model === null) {
    return harness;
  }

  return id !== null && known(id) && brands(harness, model)
    ? model
    : `${harness} ${model}`;
}

/// Whether a model's name already names its backend's brand: the first word of
/// the backend's name, as a whole word and in whatever case the model spells it.
///
/// The first word rather than the whole name, because the brand is what a model
/// is named after and the rest is the product — "Grok 4.6" is a Grok Build
/// model and says so, while "GPT-5.1 Codex" is an OpenCode one and does not.
/// A whole word rather than a substring, so that a "Codex" anywhere in a name
/// counts and a "codexed" would not.
function brands(harness: string, model: string): boolean {
  return new RegExp(`\\b${harness.split(" ")[0]!}\\b`, "i").test(model);
}

/// Whether the Profile's own name is what tells this reading apart from another
/// under the same backend.
///
/// It is not, and only, where the backend has exactly one saved Profile and this
/// is it. Everything else says the name:
///
/// - a backend with two accounts, where the name is the whole of the difference;
/// - a Pairing with no model, where the name is all there is to say;
/// - a run whose backend was never recorded, there being nothing to count;
/// - and a recorded name that no longer matches any saved Profile, where
///   dropping it would read as the account that happens to be left.
function tells(said: Said, saved: ProfileEntry[] | undefined): boolean {
  if (said.agent === null || !said.model) {
    return true;
  }

  const backend = (saved ?? []).filter(
    (profile) => profile.account.agent_type === said.agent,
  );

  return backend.length !== 1 || backend[0]!.name !== said.profile;
}
