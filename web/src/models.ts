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
//! **Each harness declares its own** (ADR-0030, *Agent*): the list a Profile of
//! it is offered, and the short names it answers to, come with it on the
//! harness list the server gives — see [`./harnesses.ts`](./harnesses.ts) — so
//! a plugin harness's models are offered as a built-in's are, and nothing here
//! is keyed by a closed list of harnesses.
//!
//! **The list goes stale.** It goes stale the week another model ships, so
//! nothing is built on the list being complete. An id the list has not learned
//! yet is picked up by hand wherever models are chosen, and [`prettify`] hands
//! back an unknown id unchanged rather than refusing it or drawing a blank: an
//! unknown model reads as `claude-opus-7` on the button, which is legible,
//! where the alternatives read as nothing at all.
//!
//! **And an id is not only ever a pick.** A harness answers to short names of
//! its own — `opus`, `fable` — and to a suffix on either spelling asking for the
//! long context, so a profile filled in by hand holds ids no list of picks would
//! have offered. Two readings stand beside the entries for that: the aliases a
//! harness declares, which are recognised and never offered, and the `[1m]`
//! rule, which reads a variant off whatever its base reads as. Both recognise
//! rather than guess — an id neither of them knows still degrades to itself.

import type { AgentType, HarnessModel } from "./api/types";
import { harness, harnesses } from "./harnesses";

/// The models a Profile of `agent` is offered, in the order its harness
/// declares them — none, for a harness this server has not got.
export function offeredFor(agent: AgentType): HarnessModel[] {
  return harness(agent)?.models ?? [];
}

/// What each id any harness here declares reads as — its models and its
/// aliases alike — by id.
function names(): Map<string, string> {
  return new Map(
    harnesses().flatMap((entry) =>
      [...entry.models, ...entry.aliases].map(
        (model) => [model.id, model.name] as const,
      ),
    ),
  );
}

/// The suffix a long-context variant is spelled with, and the words it adds.
///
/// A rule rather than a row apiece, so that recognition outlives the list:
/// whatever `claude-opus-9` comes to read as one day, the `[1m]` of it reads as
/// that and this note after it. The words are the harness's own.
const WIDE = "[1m]";
const WIDER = " (1M context)";

/// What one id reads as, or nothing where none of the three ways of knowing it
/// answers: the models, the aliases, and the `[1m]` of either.
function reads(id: string): string | undefined {
  const straight = names().get(id);

  if (straight !== undefined) {
    return straight;
  }

  if (!id.endsWith(WIDE)) {
    return undefined;
  }

  const base = reads(id.slice(0, -WIDE.length));

  return base === undefined ? undefined : `${base}${WIDER}`;
}

/// Whether this build can read the id at all — which is what says one came in by
/// hand and out of nothing the viewer recognises.
export function known(id: string): boolean {
  return reads(id) !== undefined;
}

/// One model id, as it is written where a human reads it.
///
/// The id itself for anything the list does not know, unchanged and untrimmed:
/// this is the fallback the staleness of the list is carried on, so it degrades
/// to legible rather than to empty.
export function prettify(id: string): string {
  return reads(id) ?? id;
}
