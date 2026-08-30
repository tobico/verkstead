//! The Claude models Verkstead knows the names of, and the one way an id is
//! written out for somebody to read.
//!
//! A model travels as its id everywhere it matters — a profile lists ids, a
//! pairing sends one, and the session is started under one — because the id is
//! what the agent is launched with and inventing a second name for it on the
//! wire would be inventing a second thing to be wrong. What is here is the
//! viewer's own layer over that: the ids somebody would have to type, offered as
//! picks, and the pretty name each of them reads as.
//!
//! **The list goes stale.** It goes stale the week another model ships, which is
//! the reason there was no list here for so long — so nothing is built on the
//! list being complete. An id the list has not learned yet is picked up by hand
//! wherever models are chosen, and [`prettify`] hands back an unknown id
//! unchanged rather than refusing it or drawing a blank: an unknown model reads
//! as `claude-opus-7` on the button, which is legible, where the alternatives
//! read as nothing at all.

/// One model this build knows: what it travels as, and what it reads as.
export type KnownModel = {
  /// The id the agent is launched with, and the string a profile carries.
  id: string;
  /// What a human reads instead of it.
  name: string;
};

/// The models Verkstead knows, in the order they are offered.
///
/// Ordinary first, because a picker is read from the top and these are the ones
/// a profile is likely to list.
export const KNOWN_MODELS: KnownModel[] = [
  { id: "claude-opus-5", name: "Opus 5" },
  { id: "claude-fable-5", name: "Fable 5" },
  { id: "claude-sonnet-5", name: "Sonnet 5" },
  { id: "claude-haiku-4-5-20251001", name: "Haiku 4.5" },
];

/// What each known id reads as, by id.
const NAMES: Map<string, string> = new Map(
  KNOWN_MODELS.map((model) => [model.id, model.name]),
);

/// Whether the list knows this id at all — which is what says an id came in by
/// hand rather than off a pick.
export function known(id: string): boolean {
  return NAMES.has(id);
}

/// One model id, as it is written where a human reads it.
///
/// The id itself for anything the list does not know, unchanged and untrimmed:
/// this is the fallback the staleness of the list is carried on, so it degrades
/// to legible rather than to empty.
export function prettify(id: string): string {
  return NAMES.get(id) ?? id;
}
