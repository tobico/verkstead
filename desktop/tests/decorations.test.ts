//! What the window is made with in place of a title bar.
//!
//! Three things worth pinning, and none of them can be asked of a window: the
//! module that holds one cannot be run here at all, so what it is handed is what
//! there is to check. The style, because the option beside it — `frame: false` —
//! looks like the same thing and gives a window with no overlay at all on Linux;
//! the height, because it is the one value a reader will want to know the
//! arithmetic behind; and the colours, because an overlay is a strip of solid
//! paint welded to the corner of the window and a wrong one is the first thing
//! anybody sees.

import { describe, expect, it } from "vitest";

import { BAND, DECORATIONS, INK, PAPER } from "../src/decorations.js";

describe("the title bar", () => {
  /// The whole of the choice this task was about. `frame: false` gives a window
  /// whose page reports no overlay, a titlebar rectangle of zeroes and a
  /// `setTitleBarOverlay` that throws; `"hidden"` gives a working one, and is
  /// also the option a Mac takes.
  it("is hidden rather than framed away", () => {
    expect(DECORATIONS.titleBarStyle).toBe("hidden");
  });

  /// Said as an absence because that is how it would come back: an option that
  /// crept in beside the style would take the overlay away on Linux without
  /// taking anything visible away anywhere else.
  it("says nothing about the frame", () => {
    expect(DECORATIONS).not.toHaveProperty("frame");
  });

  /// One value rather than two that have to be remembered together — a window
  /// given the style without the overlay is a bare strip with nothing in it.
  it("carries the overlay with it", () => {
    expect(DECORATIONS.titleBarOverlay).toEqual({
      color: PAPER,
      symbolColor: INK,
      height: BAND,
    });
  });
});

describe("the overlay's band", () => {
  /// The head's band at a sixteen-pixel root: a rem and a quarter of chrome
  /// padding less the rem the head hangs back up into it, the head's rem above
  /// the row and its rem below, and a row as tall as the icon buttons in it.
  it("stands about as tall as a pane head", () => {
    expect(BAND).toBe(75);
  });

  /// Electron measures the overlay in whole pixels, and a fraction handed to it
  /// is a rectangle the page then reads back rounded. The page's own push is
  /// held to the same thing from stage 04's fourth task.
  it("is a whole number of pixels", () => {
    expect(Number.isInteger(BAND)).toBe(true);
    expect(BAND).toBeGreaterThan(0);
  });
});

describe("the overlay's colours", () => {
  /// The light scheme's `--paper` and `--ink`, which is what a pane's header is
  /// drawn on and what its title is drawn in. Pinned as the values rather than
  /// as a shape, because the point of them is that they are the viewer's own.
  it("are the light scheme's paper and ink", () => {
    expect(PAPER).toBe("#faf8f5");
    expect(INK).toBe("#1c1a17");
  });

  /// `#rrggbb`, which is what Electron parses and what the page resolves its
  /// tokens to. Anything else is an overlay that does not draw.
  it("are written the way Electron reads a colour", () => {
    for (const colour of [PAPER, INK]) {
      expect(colour, colour).toMatch(/^#[0-9a-f]{6}$/);
    }
  });

  /// And they are two colours rather than one: the symbols are drawn on the
  /// ground, and an overlay whose ink was its paper would be three controls
  /// nobody can see.
  it("are not the same colour", () => {
    expect(PAPER).not.toBe(INK);
  });
});
