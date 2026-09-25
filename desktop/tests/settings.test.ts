//! The app's own settings file: what comes back from it, and what a file that
//! is not what this wrote comes back as.
//!
//! The failure this is here to catch is the all-or-nothing reading: a hand that
//! edited the file and mistyped one line has said nothing about the other, and
//! a read that threw the whole object away over it would turn one typo into two
//! surprises — a tray that went off because the close position was misspelt.

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from "vitest";

import { DEFAULTS, FILE, position, POSITIONS, set, type Settings, settings } from "../src/settings.js";

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
