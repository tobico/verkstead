//! The brand mark of the harness a session runs under, beside the words that
//! name it.
//!
//! The reading in [`./agents.ts`](./agents.ts) says who runs a session — "Claude
//! Code Fable 5 — Work" — and a mark in front of it is what makes a list of them
//! scannable: a reader picks the Claude row out of five by its shape long before
//! reading any of the words. So the mark is not decoration beside the reading, it
//! is the reading's first character, and it goes wherever the reading is drawn.
//!
//! ## Where the art comes from
//!
//! The harness's own, as the server's harness list carries it — see
//! [`./harnesses.ts`](./harnesses.ts). A plugin ships its mark as an SVG file,
//! and the built-ins' are lobehub's, kept beside the server's harnesses with
//! their licence (`crates/server/src/harness.rs` says where they came from).
//!
//! ## Drawn as an image, never put into the page
//!
//! A mark is a file somebody else wrote, so nothing in it is let run: it is
//! drawn as a CSS image of the document, which a browser renders without its
//! scripts, its links or its reach into the page around it. Nothing is said to a
//! screen reader either — the reading beside it is the whole of what the mark
//! means.
//!
//! **And in the ink around it, where it is painted in `currentColor`.** An image
//! has no ink of its own to inherit, so a mark that asks for `currentColor` — the
//! mono cut, four of the five built-ins' — is drawn as a mask over the line's own
//! colour instead: the ordinary ink on a card's second line, the heading's own in
//! the pane that card opens. A mark that names its own colours — Claude Code's
//! orange — is drawn as the picture it is.

import { Show, type JSX } from "solid-js";

import type { AgentType } from "./api/types";
import { harness } from "./harnesses";
import styles from "./HarnessMark.module.css";

export function HarnessMark(props: {
  /// The harness whose mark to draw. `null` — a run recorded before Verkstead
  /// wrote the harness down — draws nothing at all, exactly as it composes no
  /// harness word: no element, so no gap where a mark would have been. Nor does
  /// a harness this server has not got, there being no mark to draw.
  of: AgentType | null;

  /// Where it stands in the line it was drawn into. Styled by whoever passes
  /// it, as an [`Icon`](./Icon.tsx) is — the space after a mark belongs to the
  /// line, which is a flex row in one place and a run of text in three others.
  class?: string;
}): JSX.Element {
  const mark = (): string | undefined =>
    props.of === null ? undefined : harness(props.of)?.mark;

  return (
    <Show when={mark()}>
      {(svg) => (
        <span
          class={[
            styles.mark,
            svg().includes("currentColor") ? styles.inked : styles.coloured,
            props.class,
          ]
            .filter(Boolean)
            .join(" ")}
          aria-hidden="true"
          style={{ "--mark": image(svg()) }}
        />
      )}
    </Show>
  );
}

/// The mark as a CSS image of the document, which a browser draws without
/// running anything in it.
function image(svg: string): string {
  return `url("data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}")`;
}
