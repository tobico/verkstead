//! Where the window was last time, and where it can go this time.
//!
//! **Remembered per machine, in the app's own user data** (ADR-0020). Where a
//! window sits is a fact about the desk in front of the human rather than about
//! their Verkstead: the same account read from a laptop and from a desktop
//! wants two different rectangles, and neither of them is anything the server
//! has an opinion on. So this is a file of the app's beside Electron's own
//! state, and nothing about it goes on the wire — the same split the per-device
//! push switch already made.
//!
//! **And a remembered place is a suggestion rather than an instruction.**
//! Displays come and go: a window put on the second monitor of a docked laptop
//! is, on the train the next morning, at coordinates no display on this machine
//! covers — and a window opened there is a window nobody can reach, with no
//! way back to it but deleting a file they would have to be told about. So the
//! rectangle is fitted to the displays this machine has *now*, and one that
//! lands on none of them comes back the size it was in the middle of the screen.
//!
//! Everything here is a function of values — the file's contents, a list of
//! displays — so the machine with three monitors and the machine with none are
//! both ordinary unit tests. The one read of the real screen is in
//! [`window.ts`](./window.js), which is where Electron is allowed.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

import { say } from "./log.js";

/// What the state file is called inside the app's user data. Named for what it
/// holds rather than for the app, which the directory around it is already
/// named for.
export const FILE = "window.json";

/// How much of the window has to be on a display for it to count as somewhere
/// the human can reach: a strip of the top edge wide enough to take hold of and
/// deep enough to hit. A window overlapping a display by a corner pixel is off
/// the screen in every way that matters.
const ENOUGH = { width: 120, height: 32 };

/// A rectangle, in the coordinates Electron lays its displays out in.
export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

/// How big a window is, with nothing said about where.
export interface Size {
  width: number;
  height: number;
}

/// A display, as much of one as this module has any business reading.
///
/// The **work area** rather than the whole display: the part left over once the
/// platform's own furniture — a taskbar, a dock, a panel — has had its share.
/// A window remembered against the full rectangle would come back under the
/// Windows taskbar a little further each run.
export interface Display {
  workArea: Bounds;
}

/// Where to open the window: always a size, and a position only where there is
/// one worth using.
///
/// No `x` and no `y` is the platform asked to place the window itself, which is
/// the middle of the display it would choose — the right answer both for a
/// first run and for a window whose display has gone.
export interface Placement extends Size {
  x?: number;
  y?: number;
}

/// Where the window should open, given what was remembered and what displays
/// this machine has.
export function fit(
  remembered: Bounds | undefined,
  displays: readonly Display[],
  fallback: Size,
): Placement {
  if (remembered === undefined) {
    return { ...fallback };
  }

  // Somewhere the human can see it: the window comes back exactly as it was,
  // which is the whole of what remembering is for.
  if (displays.some((display) => visible(remembered, display.workArea))) {
    return { ...remembered };
  }

  // And otherwise the size is kept and the place is not. Trimmed to a display
  // this machine actually has, because the run that is missing a display is
  // usually the run on the smaller screen — a window remembered at the size of
  // a 4K monitor, opened on a laptop, is a window whose bottom half is off the
  // bottom of it.
  const room = largest(displays);

  say("the window's remembered place is off every display, so it opens in the middle of one");

  return room === undefined
    ? { width: remembered.width, height: remembered.height }
    : {
        width: Math.min(remembered.width, room.width),
        height: Math.min(remembered.height, room.height),
      };
}

/// Whether enough of `window` falls inside `area` for somebody to reach it.
function visible(window: Bounds, area: Bounds): boolean {
  const across = overlap(window.x, window.width, area.x, area.width);
  const down = overlap(window.y, window.height, area.y, area.height);

  return across >= ENOUGH.width && down >= ENOUGH.height;
}

/// How much two spans share along one axis, which is never less than nothing.
function overlap(at: number, length: number, other: number, otherLength: number): number {
  return Math.max(0, Math.min(at + length, other + otherLength) - Math.max(at, other));
}

/// The roomiest work area this machine has, or `undefined` where it reports no
/// displays at all — which is a machine the window has nothing to be fitted to.
function largest(displays: readonly Display[]): Size | undefined {
  let room: Size | undefined;

  for (const { workArea } of displays) {
    if (room === undefined || workArea.width * workArea.height > room.width * room.height) {
      room = { width: workArea.width, height: workArea.height };
    }
  }

  return room;
}

/// How far this desktop's idea of a rectangle sits from the one it was given.
///
/// **Because asking for a rectangle and reading one back is not a round trip.**
/// A window manager frames a window, and on X11 Electron gives it the frame's
/// rectangle and reads back one measured from a different corner of the same
/// frame: ask openbox for 900 by 600 at 300,150 and what comes back is 905 by
/// 605 at 299,149 — every time, by the same amount. A run that wrote down what
/// came back and opened there next time would be a window that grew five pixels
/// and crept one pixel up and to the left at every launch, which after a
/// fortnight of launches is a window nobody put where it is.
///
/// So the difference is measured once, against the rectangle this run asked
/// for, and taken back off everything written down. On a desktop that answers
/// faithfully every part of it is zero and none of this does anything.
export interface Drift {
  x: number;
  y: number;
  width: number;
  height: number;
}

/// A desktop that gives back the rectangle it was given.
export const STILL: Drift = { x: 0, y: 0, width: 0, height: 0 };

/// How far from a frame a difference can be before it is not a frame.
///
/// A title bar and a border are tens of pixels. A window that came back a
/// hundred pixels from where it was asked for was *placed* rather than framed —
/// a tiling desktop, or one that fitted an oversized window to the screen — and
/// what it did is not a difference to take off anything.
const FRAMING = 100;

/// What this desktop made of the rectangle the window was opened at.
///
/// The position counts only where one was asked for: a window the platform
/// placed itself was not asked for a position, so there is nothing its answer
/// is far from. The next run asks for one, and from then on there is.
export function drifted(asked: Placement, reported: Bounds): Drift {
  const drift = {
    x: asked.x === undefined ? 0 : reported.x - asked.x,
    y: asked.y === undefined ? 0 : reported.y - asked.y,
    width: reported.width - asked.width,
    height: reported.height - asked.height,
  };

  return Object.values(drift).some((far) => Math.abs(far) > FRAMING) ? STILL : drift;
}

/// The rectangle to write down: what the window reports, less what this desktop
/// adds to everything it reports.
export function asGiven(reported: Bounds, drift: Drift): Bounds {
  return {
    x: reported.x - drift.x,
    y: reported.y - drift.y,
    width: reported.width - drift.width,
    height: reported.height - drift.height,
  };
}

/// What the last run left at `file`, or `undefined` where there is nothing
/// usable there.
///
/// A file that is not there is a first run; a file that is there and is not
/// what this wrote is a hand that edited it, a disk that half-wrote it, or a
/// version of this app that kept something else. One answer for all of them,
/// and it is the answer a first run gets: the state file is a convenience, and
/// an app that refused to open a window over one would have made it a
/// requirement.
export function remembered(file: string): Bounds | undefined {
  let written: string;

  try {
    written = readFileSync(file, "utf8");
  } catch {
    return undefined;
  }

  let read: unknown;
  try {
    read = JSON.parse(written);
  } catch {
    say(`the remembered window bounds in ${file} are not readable, so the window opens fresh`);
    return undefined;
  }

  return bounds(read);
}

/// `read` where it is the rectangle this module writes, and nothing otherwise.
///
/// Whole numbers because that is what a screen is measured in and what Electron
/// hands back, and a positive size because a window 0 wide is one that is not
/// there. The position is unbounded on purpose: a display to the left of the
/// primary one has negative coordinates, and that is an ordinary desk.
function bounds(read: unknown): Bounds | undefined {
  if (typeof read !== "object" || read === null) {
    return undefined;
  }

  const { x, y, width, height } = read as Record<string, unknown>;

  if (!place(x) || !place(y) || !size(width) || !size(height)) {
    return undefined;
  }

  return { x, y, width, height };
}

/// Whether a value is a coordinate: a whole number, anywhere on the line.
function place(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value);
}

/// Whether a value is a length: a whole number of pixels there is some of.
function size(value: unknown): value is number {
  return place(value) && value > 0;
}

/// Keep `where` for the next run.
///
/// Never throws: a state file that cannot be written is a window that opens in
/// the middle next time, which is a smaller thing than an app that fell over
/// while being closed. The directory is made because a first run's user data
/// may not exist yet — Electron makes it for what it keeps there itself, and
/// this does not wait to find out whether it has got round to it.
export function remember(file: string, where: Bounds): void {
  try {
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, `${JSON.stringify(where)}\n`, "utf8");
  } catch (trouble) {
    say(`the window's place could not be kept in ${file} — ${String(trouble)}`);
  }
}
