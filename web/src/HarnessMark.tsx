//! The brand mark of the harness a session runs under, beside the words that
//! name it.
//!
//! The reading in [`./agents.ts`](./agents.ts) says who runs a session — "Claude
//! Code Fable 5 — Work" — and a mark in front of it is what makes a list of them
//! scannable: a reader picks the Claude row out of five by its shape long before
//! reading any of the words. So the mark is not decoration beside the reading, it
//! is the reading's first character, and it goes wherever the reading is drawn
//! and JSX can put an `svg`.
//!
//! ## Where the art comes from
//!
//! [lobehub/lobe-icons](https://github.com/lobehub/lobe-icons), MIT licensed,
//! copied into this repository rather than depended on. Its React package cannot
//! run in a SolidJS app and carries the drawing inside each component; the
//! `@lobehub/icons-static-svg` sibling publishes the same art as plain files, and
//! those four files are what sit in [`./marks/`](./marks) under lobehub's own
//! names — so updating one is copying it again. Its licence sits beside them, in
//! [`./marks/LICENSE`](./marks/LICENSE), MIT asking for the notice to travel with
//! the copy.
//!
//! Verbatim but for one edit: each file's `<title>` is taken out. It is what a
//! browser draws as a tooltip on hover, and a mark whose tooltip said "Claude"
//! beside words already reading *Claude Code Fable 5 — Work* would be the line
//! saying itself twice. Nothing here is said to a screen reader either, for the
//! same reason — the reading beside it is the whole of what the mark means.
//!
//! ## Colour, and the one mark that keeps its own
//!
//! Three of the four are the mono cut, drawn the way [`./Icon.tsx`](./Icon.tsx)
//! draws a Font Awesome icon: `currentColor` and `1em`, so a mark sits in
//! whatever ink surrounds it — soft on the status button's second line, the
//! heading's own on a card. Claude Code's is lobehub's colour variant, which
//! names its fill and so stands in its own orange wherever it is drawn. That was
//! the pick: colour where lobehub has it, and lobehub has it for Claude alone of
//! these four.

import { Show, type JSX } from "solid-js";

import type { AgentType } from "./agents";
import styles from "./HarnessMark.module.css";

import claude from "./marks/claude-color.svg?raw";
import codex from "./marks/codex.svg?raw";
import grok from "./marks/grok.svg?raw";
import opencode from "./marks/opencode.svg?raw";

/// Which drawing belongs to which harness.
///
/// Keyed by the [`AgentType`] the reading is composed from, so a fifth backend
/// arrives here by not compiling until it has been given a mark — the same way
/// `AGENT_NAME` will not compile until it has been given a name.
const MARK: Record<AgentType, string> = {
  Claude: claude,
  Codex: codex,
  Grok: grok,
  OpenCode: opencode,
};

export function HarnessMark(props: {
  /// The harness whose mark to draw. `null` — a run recorded before Verkstead
  /// wrote the harness down — draws nothing at all, exactly as it composes no
  /// harness word: no element, so no gap where a mark would have been.
  of: AgentType | null;

  /// Where it stands in the line it was drawn into. Styled by whoever passes
  /// it, as an [`Icon`](./Icon.tsx) is — the space after a mark belongs to the
  /// line, which is a flex row in one place and a run of text in three others.
  class?: string;
}): JSX.Element {
  return (
    <Show when={props.of}>
      {(agent) => (
        <span
          class={[styles.mark, props.class].filter(Boolean).join(" ")}
          aria-hidden="true"
          // The file as it was copied, which is already an `svg` sized in `em`
          // and painted in `currentColor`. Nothing is parsed out of it and
          // nothing is put back: what the bundle carries is what lobehub drew.
          innerHTML={MARK[agent()]}
        />
      )}
    </Show>
  );
}
