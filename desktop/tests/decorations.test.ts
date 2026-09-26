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
//!
//! And then the push the page makes of all three, which is the same three values
//! arriving from the other direction: the shape checked rather than trusted, the
//! translation into Electron's own words, and the platform that has no overlay for
//! any of it to reach. The call that wears it is `main.ts`'s, a window being the
//! one thing that cannot be handed in here.

import { describe, expect, it } from "vitest";

import {
  BAND,
  DECORATIONS,
  type Head,
  INK,
  OPENS,
  overlaid,
  overlay,
  PAPER,
  worn,
} from "../src/decorations.js";

/// A head the page might push: the dark scheme's paper and ink, at a band no
/// constant in this package would have guessed.
const PUSHED: Head = { paper: "#171614", ink: "#ece7e0", band: 93 };

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

describe("what the window opens wearing", () => {
  /// The same three values a push carries, which is what makes the overlay the
  /// window is made with and the overlay a push leaves behind one thing rather
  /// than two — and what keeps the moment before the page loads from being a
  /// different shape of answer to the moment after it.
  it("is a head like any the page pushes", () => {
    expect(OPENS).toEqual({ paper: PAPER, ink: INK, band: BAND });
    expect(worn(OPENS)).toEqual(OPENS);
  });

  /// Which is also the arithmetic the page redoes for itself against its own rem —
  /// see `band` in `web/src/head.ts`, and that suite for the other half of this.
  it("is what the head's band comes to at a sixteen-pixel root", () => {
    expect(OPENS.band).toBe(75);
  });
});

describe("the head as a window wears it", () => {
  /// The one place the page's vocabulary and Electron's meet: a `color` that is a
  /// paper, a `symbolColor` that is an ink, and a height that is a band.
  it("is the page's three values in Electron's three words", () => {
    expect(overlay(PUSHED)).toEqual({
      color: "#171614",
      symbolColor: "#ece7e0",
      height: 93,
    });
  });

  /// Said as an absence, because the extra key is how this would go wrong: an
  /// option Electron does not know is an option it ignores, so a translation that
  /// grew one would be an overlay quietly not doing what the page asked.
  it("says nothing else at all", () => {
    expect(Object.keys(overlay(PUSHED)).sort()).toEqual([
      "color",
      "height",
      "symbolColor",
    ]);
  });
});

describe("a head pushed over the bridge", () => {
  /// The ordinary case: the page says what it is drawn in and how tall its band
  /// stands, and that is what the overlay is made of.
  it("is what the page said, where what it said is one", () => {
    expect(worn(PUSHED)).toEqual(PUSHED);
  });

  /// Electron measures the overlay in whole pixels, and the page's own arithmetic
  /// — rules written in rem against whatever a rem is on that machine — lands on
  /// a fraction as readily as on an integer. So a fraction is rounded rather than
  /// thrown away: what has to be true of a band is that it is a number of pixels.
  it("has its band rounded to the pixel Electron measures in", () => {
    expect(worn({ ...PUSHED, band: 74.8 })?.band).toBe(75);
    expect(worn({ ...PUSHED, band: 0.5 })?.band).toBe(1);
  });

  /// And the rounding happens before the refusing rather than after it, which is
  /// the one place the two could have disagreed: a band of `0.4` is more than
  /// nothing and rounds to nothing, so rounded second it would have become the
  /// strip of no height a band of `0` is refused for.
  it("is nothing where the band rounds away to no height at all", () => {
    expect(worn({ ...PUSHED, band: 0.4 })).toBeUndefined();
  });

  /// And what is not a head changes nothing, the same reading `changed` makes of
  /// a set: a bridge is not to be trusted with a shape just because the window is
  /// the app's own, and what an unchecked push reaches is a call that throws at a
  /// colour it cannot parse.
  it("is nothing at all where what arrived is not one", () => {
    for (const pushed of [
      undefined,
      null,
      "#171614",
      93,
      [],
      {},
      { paper: "#171614", ink: "#ece7e0" },
      { paper: "#171614", band: 93 },
      { ink: "#ece7e0", band: 93 },
    ]) {
      expect(worn(pushed), JSON.stringify(pushed) ?? "undefined").toBeUndefined();
    }
  });

  /// A colour is `#rrggbb` or it is not a colour: anything else is an overlay
  /// Electron refuses to paint, and a refusal is not what a push is for.
  it("is nothing where either colour is not one Electron parses", () => {
    for (const paper of ["", "#fff", "faf8f5", "rgb(250, 248, 245)", "paper", "#gggggg"]) {
      expect(worn({ ...PUSHED, paper }), paper).toBeUndefined();
      expect(worn({ ...PUSHED, ink: paper }), paper).toBeUndefined();
    }
  });

  /// Upper case is the same colour. The page resolves its own tokens rather than
  /// copying them out of a stylesheet, so which case it lands in is the browser's
  /// affair and not something to refuse an overlay over.
  it("reads a colour in either case", () => {
    expect(worn({ ...PUSHED, paper: "#171614".toUpperCase() })?.paper).toBe("#171614");
  });

  /// And a band that is not a positive number of pixels is a strip of no height or
  /// of nonsense, either of which is worse than the overlay the window opened at.
  it("is nothing where the band is not a number of pixels", () => {
    for (const band of [0, -1, Number.NaN, Number.POSITIVE_INFINITY, "75", null]) {
      expect(worn({ ...PUSHED, band }), String(band)).toBeUndefined();
    }
  });
});

describe("the platforms an overlay is drawn on", () => {
  /// Windows and Linux, which is where `titleBarOverlay` means anything: the
  /// controls are drawn over the top-right corner of the page and a push is what
  /// recolours them.
  it("are the two the controls overlay is", () => {
    expect(overlaid("linux")).toBe(true);
    expect(overlaid("win32")).toBe(true);
  });

  /// And a Mac is not one of them. It has traffic lights rather than an overlay,
  /// and `setTitleBarOverlay` answers *"Titlebar overlay is not enabled"* with a
  /// throw there — so a push from a Mac's page is answered by doing nothing rather
  /// than by an error crossing back over the bridge. The page pushes all the same,
  /// having no platform branch in it.
  it("do not include a Mac, which has traffic lights instead", () => {
    expect(overlaid("darwin")).toBe(false);
  });
});
