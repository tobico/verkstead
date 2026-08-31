//! The document a share is built from, and the two slots the server fills.
//!
//! `web/share.html` is a template rather than a page: the share build folds the
//! script and the stylesheets into it, and the server writes a Conversation —
//! and, where the record has a Diagram in it, the diagram renderer — into two
//! empty `<script>` tags on the way out. See `crates/server/src/sharing.rs`,
//! which is the other end of it.
//!
//! **The two ends spell those tags out separately.** `RECORD` and `DIAGRAMS`
//! over there are string constants matched against this document; the tags are
//! written here. Nothing composed the two: the server's own tests fill a
//! template written by hand in the test, and the Rust suite never builds the
//! viewer at all — so an attribute reordered here, or a tag renamed, would leave
//! every download and every publish refusing with *the share build of the viewer
//! has nowhere to put a conversation*, and the first person to find out would be
//! whoever pressed Share on a release.
//!
//! So this is the comparison, from the side that owns the document. It is worth
//! nothing on its own and everything beside the constants it quotes: change
//! either end and this is what says the other has to move too.

import { describe, expect, it } from "vitest";

/// The template exactly as it is committed. Read as text rather than parsed:
/// what the server does with it is `str::find`, so what has to hold is the
/// bytes.
import TEMPLATE from "../share.html?raw";

/// `RECORD` in `crates/server/src/sharing.rs` — where the Conversation goes.
const RECORD = '<script id="share" type="application/json">';

/// And `DIAGRAMS` beside it, where the renderer goes on the shares that carry a
/// Diagram.
const DIAGRAMS = '<script id="diagrams">';

/// And `CLOSES`, which is what the server writes each of them up to: it fills a
/// slot by replacing everything between the opening tag and the next one of
/// these.
const CLOSES = "</script>";

/// How many times `what` occurs in the template.
const times = (what: string): number => TEMPLATE.split(what).length - 1;

describe("the document a share is built from", () => {
  /// Byte for byte, because that is how the server finds them: an attribute in
  /// the other order is a tag this document has and the server does not.
  it("carries the two slots the server writes into", () => {
    expect(TEMPLATE).toContain(RECORD);
    expect(TEMPLATE).toContain(DIAGRAMS);
  });

  /// One of each. The server fills the first it finds, so a second would be a
  /// slot nothing ever writes to and a reader would never know which they were
  /// looking at.
  it("carries one of each and no more", () => {
    expect(times(RECORD)).toBe(1);
    expect(times(DIAGRAMS)).toBe(1);
  });

  /// And each is closed before the next one opens, which is what makes filling
  /// one of them a replacement rather than a swallowing: the server writes up to
  /// the next `</script>`, so a slot left open would take the document with it.
  it("closes each slot before the next tag opens", () => {
    for (const [name, slot] of [
      ["the record", RECORD],
      ["the renderer", DIAGRAMS],
    ] as const) {
      const opens = TEMPLATE.indexOf(slot) + slot.length;
      const closes = TEMPLATE.indexOf(CLOSES, opens);

      expect(closes, `${name} is never closed`).toBeGreaterThan(-1);
      expect(
        TEMPLATE.slice(opens, closes).includes("<script"),
        `${name} swallows the tag after it`,
      ).toBe(false);
    }
  });

  /// The record slot holds `null` and the renderer's holds nothing.
  ///
  /// Which is what makes the untouched template a page that opens rather than
  /// one that dies on a parse: a share with no Conversation written into it says
  /// so in its own words — see `boarded` in `src/share/bundle.ts`.
  it("stands as a page before anything is written into it", () => {
    const record = TEMPLATE.indexOf(RECORD) + RECORD.length;
    expect(TEMPLATE.slice(record, TEMPLATE.indexOf(CLOSES, record))).toBe(
      "null",
    );

    const diagrams = TEMPLATE.indexOf(DIAGRAMS) + DIAGRAMS.length;
    expect(TEMPLATE.slice(diagrams, TEMPLATE.indexOf(CLOSES, diagrams))).toBe(
      "",
    );
  });
});
