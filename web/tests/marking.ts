//! Reading the harness's brand mark off whatever has drawn one — see
//! `src/HarnessMark.tsx`.
//!
//! What is asserted through here is the drawing rather than a class: lobehub's
//! four files are read as they were published, so a test naming a mark and the
//! component drawing it are two independent statements about the same art, and a
//! mark drawn out of the wrong file fails even though both would carry the same
//! class.
//!
//! Shared because the mark is: it goes wherever the reading goes — the timeline
//! card, the pane it opens, the status button, the Brief's pairing facts, and
//! every row of the five pickers — and each of those is asked about in whichever
//! file its page is tested in.

import styles from "../src/HarnessMark.module.css";

/// The path inside one of lobehub's files, which is the whole of the drawing:
/// all four are a single path in a 24-square box.
export function art(file: string): string {
  return / d="([^"]+)"/.exec(file)![1]!;
}

/// And the drawing a scope has actually put on the page — `null` where it drew
/// no mark at all.
export function marked(scope: Element): string | null {
  return scope.querySelector(`.${styles.mark} path`)?.getAttribute("d") ?? null;
}
