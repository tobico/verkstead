//! The app's own settings file: what comes back from it, what a file that is
//! not what this wrote comes back as, and what may be written to it.
//!
//! The two readings are opposites and both are here: a file is read one
//! setting at a time and a set arriving over the bridge is refused whole. The
//! difference is who is on the other end — a human who hand-edited a line, or
//! a program with a bug in it.
//!
//! The failure this is here to catch is the all-or-nothing reading: a hand that
//! edited the file and mistyped one line has said nothing about the other, and
//! a read that threw the whole object away over it would turn one typo into two
//! surprises — a tray that went off because the close position was misspelt.

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from "vitest";

import {
  changed,
  DEFAULTS,
  FILE,
  position,
  POSITIONS,
  set,
  type Settings,
  settings,
} from "../src/settings.js";

/// Everything a spy on the stream was handed — where the app's lines go on a
/// run that has no log file open, which is every run under vitest.
const said = (stderr: MockInstance<typeof process.stderr.write>): string =>
  stderr.mock.calls.map(([line]) => String(line)).join("");

let dir: string;
let file: string;

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "verkstead-settings-"));
  file = join(dir, FILE);
});

afterEach(() => vi.restoreAllMocks());

describe("what is set comes back", () => {
  it("reads back what was set", () => {
    const chosen: Settings = { whenClosed: "ask", trayIcon: false };

    set(file, chosen);

    expect(settings(file)).toEqual(chosen);
  });

  it("reads back every position", () => {
    for (const whenClosed of POSITIONS) {
      set(file, { whenClosed, trayIcon: true });

      expect(settings(file).whenClosed, whenClosed).toBe(whenClosed);
    }
  });

  /// The directory Electron keeps its user data in may not be there yet on the
  /// run that makes it — the same reason the remembered bounds make it.
  it("makes the directory it keeps the file in", () => {
    const deeper = join(dir, "not", "made", "yet", FILE);

    set(deeper, { whenClosed: "quit", trayIcon: false });

    expect(settings(deeper)).toEqual({ whenClosed: "quit", trayIcon: false });
  });

  /// Written to be read by a human as well as by this: the file is the one a
  /// machine's settings are hand-edited in when somebody is working out what
  /// the app is doing.
  it("writes the file as readable JSON", () => {
    set(file, { whenClosed: "ask", trayIcon: false });

    const written = readFileSync(file, "utf8");

    expect(written).toContain("\n");
    expect(written.endsWith("\n")).toBe(true);
    expect(JSON.parse(written)).toEqual({ whenClosed: "ask", trayIcon: false });
  });
});

describe("a machine that has not been told", () => {
  /// The ordinary first run: keep running in the tray, and there is a tray to
  /// keep running in.
  it("keeps running in the tray with an icon shown", () => {
    expect(DEFAULTS).toEqual({ whenClosed: "tray", trayIcon: true });
  });

  it("reads the defaults where there is no file", () => {
    expect(settings(file)).toEqual(DEFAULTS);
  });

  /// And says nothing about it: a file that is not there is a question nobody
  /// has answered rather than anything that went wrong.
  it("says nothing about a file that is not there", () => {
    const stderr = vi.spyOn(process.stderr, "write").mockReturnValue(true);

    settings(file);

    expect(said(stderr)).toBe("");
  });

  /// Handed out rather than shared: a caller that set a field on what it read
  /// would be editing the defaults every later read comes back as.
  it("hands out a set of its own each time", () => {
    const read = settings(file);
    read.trayIcon = false;

    expect(settings(file)).toEqual(DEFAULTS);
    expect(DEFAULTS.trayIcon).toBe(true);
  });
});

describe("a file that is not what this wrote", () => {
  /// One setting at a time: the good half of a hand-edited file still carries.
  it("keeps the settings it can read and defaults the rest", () => {
    for (const [written, expected] of [
      ['{"whenClosed": "quit"}', { whenClosed: "quit", trayIcon: true }],
      ['{"trayIcon": false}', { whenClosed: "tray", trayIcon: false }],
      ['{"whenClosed": "minimise", "trayIcon": false}', { whenClosed: "tray", trayIcon: false }],
      ['{"whenClosed": "ask", "trayIcon": "yes"}', { whenClosed: "ask", trayIcon: true }],
      ['{"whenClosed": null, "trayIcon": 1}', DEFAULTS],
      ['{"whenClosed": "ask", "trayIcon": false, "colour": "green"}', {
        whenClosed: "ask",
        trayIcon: false,
      }],
      ["{}", DEFAULTS],
    ] satisfies [string, Settings][]) {
      writeFileSync(file, written, "utf8");

      expect(settings(file), written).toEqual(expected);
    }
  });

  /// And a file that is no set of settings at all is the defaults whole —
  /// there is no half of it to keep.
  it("reads the defaults from a file that holds nothing to read", () => {
    const stderr = vi.spyOn(process.stderr, "write").mockReturnValue(true);

    for (const written of ["", "not json at all", "null", "[1, 2, 3]", '"tray"', "7"]) {
      writeFileSync(file, written, "utf8");

      expect(settings(file), written).toEqual(DEFAULTS);
    }

    // Said, because unlike a missing file this is something somebody did.
    expect(said(stderr)).toContain(FILE);
  });
});

describe("the positions", () => {
  it("are keep running in the tray, ask, and quit", () => {
    expect(POSITIONS).toEqual(["tray", "ask", "quit"]);
  });

  it("reads one of the three back and nothing else", () => {
    expect(POSITIONS.map(position)).toEqual([...POSITIONS]);

    for (const value of ["", "TRAY", "minimise", null, 0, true, {}]) {
      expect(position(value), JSON.stringify(value)).toBeUndefined();
    }
  });
});

describe("a set that arrived over the bridge", () => {
  it("takes one setting, or both, and says what is to change", () => {
    expect(changed({ whenClosed: "quit" })).toEqual({ whenClosed: "quit" });
    expect(changed({ trayIcon: false })).toEqual({ trayIcon: false });
    expect(changed({ whenClosed: "ask", trayIcon: true })).toEqual({
      whenClosed: "ask",
      trayIcon: true,
    });
  });

  it("takes every position", () => {
    for (const whenClosed of POSITIONS) {
      expect(changed({ whenClosed }), whenClosed).toEqual({ whenClosed });
    }
  });

  /// A renderer is a renderer: what comes up the bridge is checked against the
  /// shape before it reaches the file, whoever it came from.
  it("refuses a value that is not one of the positions or not a boolean", () => {
    for (const sent of [
      { whenClosed: "minimise" },
      { whenClosed: "TRAY" },
      { whenClosed: "" },
      { whenClosed: null },
      { whenClosed: true },
      { trayIcon: "yes" },
      { trayIcon: 1 },
      { trayIcon: null },
    ]) {
      expect(changed(sent), JSON.stringify(sent)).toBeUndefined();
    }
  });

  /// All or nothing, which is the opposite of what a hand-edited file gets: a
  /// set that is half wrong is a program with a bug in it, and writing the half
  /// that parsed would be the app guessing at what the bug meant.
  it("refuses the whole set where one half of it is wrong", () => {
    expect(changed({ whenClosed: "ask", trayIcon: "yes" })).toBeUndefined();
    expect(changed({ whenClosed: "minimise", trayIcon: false })).toBeUndefined();
  });

  /// The app has two settings, and a set naming a third is not a set of this
  /// app's settings — no more than a position it has never heard of is.
  it("refuses a key this app has no setting for", () => {
    expect(changed({ colour: "green" })).toBeUndefined();
    expect(changed({ trayIcon: false, colour: "green" })).toBeUndefined();
  });

  /// Every set this app makes comes from a control somebody moved, so a set
  /// that names nothing is nothing to write.
  it("refuses a set that is no set of settings at all", () => {
    for (const sent of [{}, null, undefined, [], ["trayIcon"], "trayIcon", 7, true]) {
      expect(changed(sent), JSON.stringify(sent)).toBeUndefined();
    }
  });

  /// What comes back is a change rather than the settings: a set naming one of
  /// them says nothing about the other, and a caller that read the missing half
  /// as a default would turn a press of the checkbox into a reset of the radio.
  it("carries only what was named", () => {
    expect(Object.keys(changed({ trayIcon: false }) ?? {})).toEqual(["trayIcon"]);
  });
});
