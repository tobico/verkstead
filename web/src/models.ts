//! The models Verkstead knows the names of, whose model each of them is, and the
//! one way an id is written out for somebody to read.
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
//!
//! **Each entry says whose model it is**, which is the one thing a flat list of
//! ids cannot be read without now that there is more than one backend: the
//! profile form offers a Claude Code account the Claude models and no others,
//! and the reading in [`./agents.ts`](./agents.ts) drops a backend's name from
//! a model whose own name has already said it.

import type { AgentType } from "./agents";

/// One model this build knows: what it travels as, what it reads as, and whose
/// it is.
export type KnownModel = {
  /// The id the agent is launched with, and the string a profile carries.
  id: string;
  /// What a human reads instead of it.
  name: string;
  /// And the backend that launches it. One apiece: the same model reachable
  /// through two of them is two entries, because the id each is launched with
  /// is that backend's own spelling of it — which is what `minimax/minimax-m2.1`
  /// is and `grok-4.6` is not.
  agent: AgentType;
};

/// The models Verkstead knows, in the order they are offered.
///
/// Grouped by backend, because the form offers one backend's at a time — and
/// ordinary first within each group, because a picker is read from the top and
/// these are the ones a profile is likely to list.
export const KNOWN_MODELS: KnownModel[] = [
  { id: "claude-opus-5", name: "Opus 5", agent: "Claude" },
  { id: "claude-fable-5", name: "Fable 5", agent: "Claude" },
  { id: "claude-sonnet-5", name: "Sonnet 5", agent: "Claude" },
  { id: "claude-haiku-4-5-20251001", name: "Haiku 4.5", agent: "Claude" },
  { id: "gpt-5-codex", name: "GPT-5 Codex", agent: "Codex" },
  { id: "grok-4.6", name: "Grok 4.6", agent: "Grok" },
  { id: "grok-4.5", name: "Grok 4.5", agent: "Grok" },
  { id: "minimax/minimax-m2.1", name: "Minimax M2.1", agent: "OpenCode" },
  { id: "opencode/gpt-5.1-codex", name: "GPT-5.1 Codex", agent: "OpenCode" },
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
