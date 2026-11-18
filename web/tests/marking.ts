//! Reading the harness's brand mark off whatever has drawn one — see
//! `src/HarnessMark.tsx`.
//!
//! What is asserted through here is the drawing rather than a class: the mark
//! files are read as they were published, so a test naming a mark and the
//! component drawing it are two independent statements about the same art, and a
//! mark drawn out of the wrong file fails even though both would carry the same
//! class.
//!
//! Shared because the mark is: it goes wherever the reading goes — the timeline
//! card, the pane it opens, the status button, the Brief's pairing facts, and
//! every row of the five pickers — and each of those is asked about in whichever
//! file its page is tested in.

import styles from "../src/HarnessMark.module.css";

/// The path inside one of the mark files, which is the whole of the drawing:
/// each is a single path in a 24-square box.
export function art(file: string): string {
  return / d="([^"]+)"/.exec(file)![1]!;
}

/// And the drawing a scope has actually put on the page — `null` where it drew
/// no mark at all. The mark is drawn as an image of its document, so what is
/// read is that document's path.
export function marked(scope: Element): string | null {
  const mark = scope.querySelector<HTMLElement>(`.${styles.mark}`);

  return mark === null ? null : art(drawing(mark));
}

/// The SVG document a mark element draws.
export function drawing(mark: Element): string {
  const image = (mark as HTMLElement).style.getPropertyValue("--mark");
  const data = /data:image\/svg\+xml;charset=utf-8,([^"]*)/.exec(image)![1]!;

  return decodeURIComponent(data);
}
