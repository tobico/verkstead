//! The log file: the mark at the head of one that is being started, and the
//! roll at the bound.
//!
//! Over a directory the test hands it, which is what makes all of this
//! ordinary Node — the writer takes a directory rather than resolving one, and
//! where the real one is is `platform.test.ts`'s question. These are the two
//! rules the Rust app set and this app keeps, so they are asked here the way
//! `crates/desktop/src/logs.rs` asks them of itself.

import { mkdtempSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it, vi, type MockInstance } from "vitest";

import { bounded, FILE, heard, keep, MARK, PREVIOUS, ROLL_AT, say } from "../src/log.js";

/// A directory of this test's own, which is the whole of what the writer needs.
const somewhere = (): string => mkdtempSync(join(tmpdir(), "verkstead-log-"));

/// What is in the live file, mark and all.
const live = (dir: string): string => readFileSync(join(dir, FILE), "utf8");

/// An event of `bytes` bytes and a newline, which is all the writer cares about
/// one.
const event = (bytes: number): string => `${"x".repeat(bytes - 1)}\n`;

/// Everything a spy on the stream was handed, which is where a run with no log
/// file puts its lines.
const said = (stderr: MockInstance<typeof process.stderr.write>): string =>
  stderr.mock.calls.map(([line]) => String(line)).join("");

afterEach(() => vi.restoreAllMocks());

describe("a file this app starts", () => {
  it("opens with the byte-order mark", () => {
    const dir = somewhere();

    bounded(dir).write("verkstead is listening\n");

    expect(live(dir).startsWith(MARK)).toBe(true);
  });

  /// Because the mark belongs to a file being *started*: a second run appending
  /// to the first one's file would otherwise put a mark in the middle of it,
  /// which every reader shows as a character rather than as an encoding.
  it("adds no mark to a file that already has content", () => {
    const dir = somewhere();
    writeFileSync(join(dir, FILE), `${MARK}the run before this one\n`);

    bounded(dir).write("this run\n");

    expect(live(dir)).toBe(`${MARK}the run before this one\nthis run\n`);
    expect(live(dir).indexOf(MARK)).toBe(live(dir).lastIndexOf(MARK));
  });

  /// A file the run before left empty — power lost between the open and the
  /// first write — is a file being started as much as one that is not there.
  it("marks a file that is there and empty", () => {
    const dir = somewhere();
    writeFileSync(join(dir, FILE), "");

    bounded(dir).write("this run\n");

    expect(live(dir)).toBe(`${MARK}this run\n`);
  });

  it("is in a directory the writer makes", () => {
    const dir = join(somewhere(), "state", "verkstead");

    bounded(dir).write("verkstead is listening\n");

    expect(live(dir)).toContain("verkstead is listening");
  });
});

describe("a file at the bound", () => {
  it("rolls over to the one behind it", () => {
    const dir = somewhere();
    const log = bounded(dir);

    const line = event(64 * 1024);
    for (let written = 0; written <= ROLL_AT; written += line.length) {
      log.write(line);
    }

    expect(statSync(join(dir, PREVIOUS)).size).toBeGreaterThan(ROLL_AT - line.length);
    expect(statSync(join(dir, FILE)).size).toBeLessThan(ROLL_AT);
    expect(live(dir).startsWith(MARK)).toBe(true);
  });

  /// At most two files and at most twice the bound, whatever the machine has
  /// been up to: the roll before this one is what the roll overwrites.
  it("keeps nothing behind the one it rolls to", () => {
    const dir = somewhere();
    const log = bounded(dir);

    const line = event(64 * 1024);
    for (let written = 0; written <= ROLL_AT * 2; written += line.length) {
      log.write(line);
    }

    expect(statSync(join(dir, PREVIOUS)).size).toBeLessThan(ROLL_AT + line.length);
    expect(statSync(join(dir, FILE)).size).toBeLessThan(ROLL_AT);
  });

  /// The whole point of rolling where a write would cross rather than where one
  /// has: a file cut through the middle of a line is a line neither file holds.
  it("is cut between lines rather than through one", () => {
    const dir = somewhere();
    const log = bounded(dir);

    const line = event(64 * 1024);
    for (let written = 0; written <= ROLL_AT; written += line.length) {
      log.write(line);
    }

    for (const file of [FILE, PREVIOUS]) {
      const written = readFileSync(join(dir, file), "utf8").replace(MARK, "");
      expect(written.endsWith("\n"), `${file} ends on a line`).toBe(true);
      expect(new Set(written.split("\n").slice(0, -1).map((said) => said.length))).toEqual(
        new Set([line.length - 1]),
      );
    }
  });

  /// An empty file does not roll: a machine whose first event is larger than
  /// the bound would otherwise shuffle nothing along and start again.
  it("does not roll a file holding nothing but its mark", () => {
    const dir = somewhere();

    bounded(dir).write(event(ROLL_AT + 1024));

    expect(statSync(join(dir, FILE)).size).toBeGreaterThan(ROLL_AT);
    expect(() => statSync(join(dir, PREVIOUS))).toThrow();
  });
});

/// The sink, which is the one piece of module state this app has: `say` and
/// `heard` are called from everywhere and the file they land in is decided
/// once, at startup. So these are asked in the order a run asks them — before
/// anything has said where to keep a log, and then after.
describe("where this run's logging goes", () => {
  it("is standard error before anything has said where to keep one", () => {
    const stderr = vi.spyOn(process.stderr, "write").mockReturnValue(true);

    say("Verkstead is already running, so this launch hands over to it");

    expect(said(stderr)).toContain("hands over to it");
  });

  it("is the log file, with the app's lines and the sidecar's in it as they happened", () => {
    // Silenced rather than read: which file it is is the test below.
    vi.spyOn(process.stderr, "write").mockReturnValue(true);
    const dir = somewhere();

    const kept = keep(dir);
    say("the sidecar is /usr/bin/verkstead");
    heard("2026-09-25T10:00:00Z  INFO verkstead_server: verkstead is listening");
    say("the server answered after 412 ms");

    expect(kept).toEqual({ file: join(dir, FILE) });
    expect(live(dir).replace(MARK, "").split("\n").slice(0, -1)).toEqual([
      expect.stringContaining(`verkstead_desktop: this run's log is ${join(dir, FILE)}`),
      expect.stringContaining("verkstead_desktop: the sidecar is /usr/bin/verkstead"),
      "2026-09-25T10:00:00Z  INFO verkstead_server: verkstead is listening",
      expect.stringContaining("verkstead_desktop: the server answered after 412 ms"),
    ]);
  });

  /// The one line that goes both ways, because a line inside the file cannot
  /// tell anybody where the file is.
  it("says which file that is on standard error as well", () => {
    const stderr = vi.spyOn(process.stderr, "write").mockReturnValue(true);
    const dir = somewhere();

    keep(dir);
    say("the server answered after 412 ms");

    expect(said(stderr)).toBe(`this run's log is ${join(dir, FILE)}\n`);
  });

  /// The sidecar writes for the shell that started it, and that shell is a
  /// pipe: what a terminal would have read as colour is what a text editor
  /// reads as gibberish, and this file is opened in a text editor.
  it("has the terminal's own colouring off the sidecar's lines", () => {
    // Silenced: which file it is is asked above.
    vi.spyOn(process.stderr, "write").mockReturnValue(true);
    const dir = somewhere();

    keep(dir);
    heard(
      "\u001B[2m2026-09-25T10:00:00Z\u001B[0m \u001B[32m INFO\u001B[0m " +
        "\u001B[2mverkstead_server\u001B[0m\u001B[2m:\u001B[0m verkstead is listening",
    );

    expect(live(dir).split("\n").at(-2)).toBe(
      "2026-09-25T10:00:00Z  INFO verkstead_server: verkstead is listening",
    );
  });

  /// Nowhere to put a log file is not a failure: a Verkstead with nowhere to
  /// keep its database has nothing to serve, and one with nowhere to keep a log
  /// has only lost the log.
  it("is standard error where the machine names nowhere to put one", () => {
    const stderr = vi.spyOn(process.stderr, "write").mockReturnValue(true);

    const kept = keep(undefined);
    say("the server answered after 412 ms");

    expect(kept).toEqual({ nowhere: expect.stringContaining("nowhere to keep a log file") });
    expect(said(stderr)).toContain("the server answered after 412 ms");
  });

  /// The other misfortune with the same answer, and the one a machine actually
  /// meets: a directory named and not makeable.
  it("is standard error where the directory cannot be made", () => {
    const stderr = vi.spyOn(process.stderr, "write").mockReturnValue(true);
    const inTheWay = join(somewhere(), "state");
    writeFileSync(inTheWay, "a file where the directory would go");

    const kept = keep(join(inTheWay, "verkstead"));
    say("the server answered after 412 ms");

    expect(kept).toEqual({ nowhere: expect.stringContaining(join(inTheWay, "verkstead")) });
    expect(said(stderr)).toContain("could not open its log file");
    expect(said(stderr)).toContain("the server answered after 412 ms");
  });
});
