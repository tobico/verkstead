//! What the window is made with in place of a title bar.
//!
//! Four things worth pinning, and none of them can be asked of a window: the
//! module that holds one cannot be run here at all, so what it is handed is what
//! there is to check. The style, because the option beside it — `frame: false` —
//! looks like the same thing and gives a window with no overlay at all on Linux;
//! the height, because it is the one value a reader will want to know the
//! arithmetic behind; the colours, because an overlay is a strip of solid paint
//! welded to the corner of the window and a wrong one is the first thing anybody
//! sees; and the point a Mac's traffic lights are moved to, which is the one of
//! the four that is arithmetic rather than a value copied across.
//!
//! And then the push the page makes of all four, which is the same four values
//! arriving from the other direction: the shape checked rather than trusted, the
//! translation into Electron's own words, and which of the two things a platform
//! has for any of it to reach. The calls that wear it are `main.ts`'s, a window
//! being the one thing that cannot be handed in here.

import { describe, expect, it } from "vitest";

import {
  BAND,
  buttoned,
  DECORATIONS,
  type Head,
  INK,
  INSET,
  LIGHTS,
  lights,
  MIDDLE,
  OPENS,
  overlaid,
  overlay,
  PAPER,
  worn,
} from "../src/decorations.js";

/// A head the page might push: the dark scheme's paper and ink, at a band and a
/// row no constant in this package would have guessed.
const PUSHED: Head = { paper: "#171614", ink: "#ece7e0", band: 93, middle: 48 };

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

  /// And the point a Mac's lights open at, which is the same value said for the
  /// platform that has no overlay: a window made without it is three buttons over
  /// the pane chrome's padding for as long as the page takes to load, and one made
  /// with a constant of its own is the arithmetic in [`lights`] written twice.
  it("carries where a Mac's traffic lights open, worked out the one way", () => {
    expect(DECORATIONS.trafficLightPosition).toEqual(lights(OPENS));
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

describe("the row the head's title stands in", () => {
  /// The front of the band's own sum, at a sixteen-pixel root: a rem and a quarter
  /// of chrome padding less the rem the head hangs back up into it, the head's rem
  /// above its row, and half of the row itself.
  it("has its middle about forty pixels down the window", () => {
    expect(MIDDLE).toBe(39);
  });

  /// Whole pixels and above nothing, the same as the band: this is half of a point
  /// a window is handed, and the page's own push is held to it too.
  it("is a whole number of pixels", () => {
    expect(Number.isInteger(MIDDLE)).toBe(true);
    expect(MIDDLE).toBeGreaterThan(0);
  });

  /// And it is not the middle of the band, which is the whole reason it crosses the
  /// bridge at all: the band carries the head's rem *below* the row and a quarter
  /// rem of chrome above it, so its middle sits a couple of pixels higher than the
  /// row's. Lights centred on the band would be lights centred on nothing a head
  /// draws.
  it("sits below the middle of the band, rather than at it", () => {
    expect(MIDDLE).toBeGreaterThan(BAND / 2);
    expect(MIDDLE).toBeLessThan(BAND);
  });
});

describe("where a Mac's traffic lights go", () => {
  /// Measured on macOS 15 under Electron 43 rather than reasoned about: each button
  /// reports a sixteen-by-sixteen frame, and the point is their top-left corner —
  /// a `y` of thirty put a button's top thirty pixels down the window.
  it("is worked out against the size the platform draws them", () => {
    expect(LIGHTS).toBe(16);
    expect(INSET).toBe(20);
  });

  /// The head's own row, centred: half a light above the middle the page pushed, at
  /// the inset the platform draws its corner at.
  it("centres them on the row the page said it drew", () => {
    expect(lights(PUSHED)).toEqual({ x: INSET, y: PUSHED.middle - LIGHTS / 2 });
    expect(lights(OPENS)).toEqual({ x: 20, y: 31 });
  });

  /// And it follows the head rather than a constant: a page drawn at a larger text
  /// size has a taller head, and a taller head is lights further down the window.
  it("moves down with a head that stands taller", () => {
    const larger = { ...PUSHED, band: 116, middle: 49 };

    expect(lights(larger).y).toBeGreaterThan(lights(OPENS).y);
    expect(lights(larger).y).toBe(41);
  });

  /// Never above the window's own top edge. A band the page says is a few pixels
  /// tall is a row whose middle is above half a light, and a negative offset is
  /// three controls hanging off the top of the window.
  it("never puts them above the top of the window", () => {
    expect(lights({ ...PUSHED, middle: 4 })).toEqual({ x: INSET, y: 0 });
    expect(lights({ ...PUSHED, middle: 1 }).y).toBe(0);
  });

  /// Said as an absence, the way the overlay's translation is: a point Electron
  /// does not know is a point it ignores, so an extra key here would be the lights
  /// quietly staying where they were.
  it("says nothing but a point", () => {
    expect(Object.keys(lights(PUSHED)).sort()).toEqual(["x", "y"]);
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
  /// The same four values a push carries, which is what makes the overlay the
  /// window is made with and the overlay a push leaves behind one thing rather
  /// than two — and what keeps the moment before the page loads from being a
  /// different shape of answer to the moment after it.
  it("is a head like any the page pushes", () => {
    expect(OPENS).toEqual({ paper: PAPER, ink: INK, band: BAND, middle: MIDDLE });
    expect(worn(OPENS)).toEqual(OPENS);
  });

  /// Which is also the arithmetic the page redoes for itself against its own rem —
  /// see `band` and `middle` in `web/src/head.ts`, and that suite for the other
  /// half of this.
  it("is what the head's band and row come to at a sixteen-pixel root", () => {
    expect(OPENS.band).toBe(75);
    expect(OPENS.middle).toBe(39);
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

  /// And the row's middle the same way, for the same reason: a point is whole
  /// pixels, and the page's arithmetic against its own rem lands where it lands —
  /// the row's middle at a sixteen-pixel root is thirty-nine and two fifths.
  it("has its row's middle rounded the same way", () => {
    expect(worn({ ...PUSHED, middle: 39.4 })?.middle).toBe(39);
    expect(worn({ ...PUSHED, middle: 48.5 })?.middle).toBe(49);
  });

  /// And the rounding happens before the refusing rather than after it, which is
  /// the one place the two could have disagreed: a band of `0.4` is more than
  /// nothing and rounds to nothing, so rounded second it would have become the
  /// strip of no height a band of `0` is refused for.
  it("is nothing where either measurement rounds away to nothing at all", () => {
    expect(worn({ ...PUSHED, band: 0.4 })).toBeUndefined();
    expect(worn({ ...PUSHED, middle: 0.4 })).toBeUndefined();
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
      { paper: "#171614", band: 93, middle: 48 },
      { ink: "#ece7e0", band: 93, middle: 48 },
      { paper: "#171614", ink: "#ece7e0", band: 93 },
      { paper: "#171614", ink: "#ece7e0", middle: 48 },
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
  /// The row's middle is held to the same thing: a point worked out of nonsense is
  /// three buttons somewhere nobody asked for.
  it("is nothing where either measurement is not a number of pixels", () => {
    for (const measured of [0, -1, Number.NaN, Number.POSITIVE_INFINITY, "75", null]) {
      expect(worn({ ...PUSHED, band: measured }), String(measured)).toBeUndefined();
      expect(worn({ ...PUSHED, middle: measured }), String(measured)).toBeUndefined();
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
  /// throw there — so a push from a Mac's page is answered by moving the buttons
  /// rather than by an error crossing back over the bridge. The page pushes all the
  /// same, having no platform branch in it.
  it("do not include a Mac, which has traffic lights instead", () => {
    expect(overlaid("darwin")).toBe(false);
  });
});

describe("the platforms whose buttons are moved instead", () => {
  /// A Mac, which is where `setWindowButtonPosition` means anything.
  it("are the one the traffic lights are", () => {
    expect(buttoned("darwin")).toBe(true);
  });

  /// And not the two that have an overlay: there the controls are drawn at the
  /// corner the platform chose and the call is not there to be made.
  it("do not include the two an overlay is drawn on", () => {
    expect(buttoned("linux")).toBe(false);
    expect(buttoned("win32")).toBe(false);
  });

  /// And no platform is both, which is what makes the two arms of a push an answer
  /// rather than a pair of them: an overlay recoloured *and* buttons moved would be
  /// one of the two calls throwing.
  it("are never a platform that has an overlay too", () => {
    for (const platform of ["darwin", "linux", "win32"] as const) {
      expect(buttoned(platform) && overlaid(platform), platform).toBe(false);
    }
  });
});
