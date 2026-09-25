//! The window's place, fitted to the displays there are.
//!
//! Two halves, and the second one is the one that matters: a rectangle read
//! back off disk, and that rectangle put against a machine whose displays may
//! not be the ones it was written on. The failure this is here to catch is the
//! window that comes back at coordinates nothing covers — which is not a window
//! in the wrong place but a window nobody can reach.

import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { beforeEach, describe, expect, it } from "vitest";

import {
  asGiven,
  type Bounds,
  type Display,
  drifted,
  FILE,
  fit,
  remember,
  remembered,
  STILL,
} from "../src/bounds.js";

/// The size the window opens at when nothing has been remembered, standing in
/// for `window.ts`'s own.
const FALLBACK = { width: 1280, height: 860 };

/// One laptop screen, with room taken off the top for a panel.
const LAPTOP: Display = { workArea: { x: 0, y: 32, width: 1920, height: 1048 } };

/// And the monitor it is docked to, to the right of it.
const MONITOR: Display = { workArea: { x: 1920, y: 0, width: 2560, height: 1440 } };

let dir: string;

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "verkstead-bounds-"));
});

describe("what is remembered comes back", () => {
  it("reads back what was kept", () => {
    const file = join(dir, FILE);
    const where: Bounds = { x: 240, y: 120, width: 1100, height: 700 };

    remember(file, where);

    expect(remembered(file)).toEqual(where);
  });

  /// A first run, which is the ordinary way to meet no file.
  it("remembers nothing where there is no file", () => {
    expect(remembered(join(dir, FILE))).toBeUndefined();
  });

  /// The directory Electron keeps its user data in may not be there yet on the
  /// run that makes it.
  it("makes the directory it keeps the file in", () => {
    const file = join(dir, "not", "made", "yet", FILE);

    remember(file, { x: 0, y: 0, width: 800, height: 600 });

    expect(remembered(file)).toEqual({ x: 0, y: 0, width: 800, height: 600 });
  });

  it("remembers nothing from a file that is not what this writes", () => {
    for (const written of [
      "",
      "not json at all",
      "null",
      "[1, 2, 3, 4]",
      '{"x": 0, "y": 0, "width": 800}',
      '{"x": 0, "y": 0, "width": 800, "height": "600"}',
      '{"x": 0, "y": 0, "width": 0, "height": 600}',
      '{"x": 0, "y": 0, "width": -800, "height": 600}',
      '{"x": 0.5, "y": 0, "width": 800, "height": 600}',
    ]) {
      const file = join(dir, FILE);
      writeFileSync(file, written, "utf8");

      expect(remembered(file), written).toBeUndefined();
    }
  });

  /// A window on a display to the left of the primary one sits at negative
  /// coordinates, and that is an ordinary desk rather than a broken file.
  it("keeps a negative position, which is the display to the left", () => {
    const file = join(dir, FILE);
    const where: Bounds = { x: -1720, y: -200, width: 1100, height: 700 };

    remember(file, where);

    expect(remembered(file)).toEqual(where);
  });
});

describe("where the window opens", () => {
  it("opens at the default size on a first run", () => {
    expect(fit(undefined, [LAPTOP, MONITOR], FALLBACK)).toEqual(FALLBACK);
  });

  it("comes back exactly where it was", () => {
    const where: Bounds = { x: 300, y: 200, width: 1100, height: 700 };

    expect(fit(where, [LAPTOP], FALLBACK)).toEqual(where);
  });

  it("comes back on the second display when the second display is there", () => {
    const where: Bounds = { x: 2400, y: 300, width: 1100, height: 700 };

    expect(fit(where, [LAPTOP, MONITOR], FALLBACK)).toEqual(where);
  });

  /// The laptop undocked: the same window, the same file, and the monitor it
  /// was on gone.
  it("comes back in the middle when the display it was on has gone", () => {
    const where: Bounds = { x: 2400, y: 300, width: 1100, height: 700 };

    expect(fit(where, [LAPTOP], FALLBACK)).toEqual({ width: 1100, height: 700 });
  });

  /// The 4K monitor gone as well as the place: a window kept at its size would
  /// have its bottom half below the bottom of the laptop screen.
  it("trims a window too big for the display it comes back on", () => {
    const where: Bounds = { x: 2000, y: 100, width: 2400, height: 1380 };

    expect(fit(where, [LAPTOP], FALLBACK)).toEqual({ width: 1920, height: 1048 });
  });

  /// Off the top of the screen is as far out of reach as off the side, and a
  /// window whose title bar is above the work area cannot be dragged back.
  it("counts a window barely overlapping a display as off it", () => {
    for (const where of [
      { x: 1910, y: 100, width: 1100, height: 700 },
      { x: -1090, y: 100, width: 1100, height: 700 },
      { x: 300, y: 1060, width: 1100, height: 700 },
      { x: 300, y: -690, width: 1100, height: 700 },
    ] satisfies Bounds[]) {
      expect(fit(where, [LAPTOP], FALLBACK), JSON.stringify(where)).toEqual({
        width: 1100,
        height: 700,
      });
    }
  });

  /// A machine reporting no displays at all is one there is nothing to fit to,
  /// and the window still has to open.
  it("opens at the remembered size on a machine with no displays", () => {
    const where: Bounds = { x: 300, y: 200, width: 1100, height: 700 };

    expect(fit(where, [], FALLBACK)).toEqual({ width: 1100, height: 700 });
    expect(fit(undefined, [], FALLBACK)).toEqual(FALLBACK);
  });
});

describe("what the desktop makes of the rectangle it is given", () => {
  /// Openbox, measured: ask for 900 by 600 at 300,150 and read back 905 by 605
  /// at 299,149 — the same five and the same one at every launch.
  const FRAMED: Bounds = { x: 299, y: 149, width: 905, height: 605 };
  const ASKED: Bounds = { x: 300, y: 150, width: 900, height: 600 };

  it("measures what a framing desktop adds", () => {
    expect(drifted(ASKED, FRAMED)).toEqual({ x: -1, y: -1, width: 5, height: 5 });
  });

  it("measures nothing on a desktop that answers faithfully", () => {
    expect(drifted(ASKED, ASKED)).toEqual(STILL);
  });

  /// The whole point: what is written down is what was asked for, so the next
  /// launch asks for the same thing and the window stays where it is.
  it("writes back what was asked for when nothing has moved", () => {
    expect(asGiven(FRAMED, drifted(ASKED, FRAMED))).toEqual(ASKED);
  });

  it("writes back what the human did, once they have done it", () => {
    const drift = drifted(ASKED, FRAMED);
    const moved: Bounds = { x: 519, y: 349, width: 1005, height: 705 };

    expect(asGiven(moved, drift)).toEqual({ x: 520, y: 350, width: 1000, height: 700 });
  });

  /// A window the platform placed for itself was asked for no position, so its
  /// answer is not far from anything.
  it("measures no position where none was asked for", () => {
    const placed = { width: 900, height: 600 };

    expect(drifted(placed, FRAMED)).toEqual({ x: 0, y: 0, width: 5, height: 5 });
  });

  /// A tiling desktop, or one that fitted an oversized window to the screen:
  /// what it did is a placement rather than a frame, and taking it off
  /// everything afterwards would be writing down a window that was never there.
  it("measures nothing where the window was placed rather than framed", () => {
    for (const reported of [
      { x: 0, y: 0, width: 900, height: 600 },
      { x: 300, y: 150, width: 1920, height: 600 },
      { x: 300, y: 150, width: 900, height: 1080 },
    ] satisfies Bounds[]) {
      expect(drifted(ASKED, reported), JSON.stringify(reported)).toEqual(STILL);
    }
  });

  it("writes back exactly what it is handed on a desktop that adds nothing", () => {
    expect(asGiven(FRAMED, STILL)).toEqual(FRAMED);
  });
});
