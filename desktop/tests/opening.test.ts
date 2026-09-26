//! Which program opens a file or a link on this machine, and what it is given.
//!
//! Two things are worth a test. **Which of the openers is picked**, because the
//! whole point of picking is `gio open`'s following of a mime subclass that
//! `xdg-open` does not — a machine that fell back where it had a `gio` is a
//! **View Logs** that opens nothing, which is the case this module exists for.
//! And **what the child is actually started with**, because a value saying which
//! program to run is worth nothing if the spawn runs another.
//!
//! The handing over is asked of a real child, the way `sidecar.test.ts` asks its
//! stand-in: a script that records what it was given is the only thing that can
//! say a `spawn` was made as the value said.

import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { hand, opening } from "../src/opening.js";

/// A look along a `PATH` that says `has` and nothing else is there.
const holding =
  (...has: string[]) =>
  (_path: string | undefined, program: string) =>
    has.includes(program);

describe("which program opens something", () => {
  /// Asked in that order and not the other: `gio` follows the subclass and
  /// `xdg-open` does not, so a machine with both is asked for `gio`.
  it("is `gio open` on a Linux machine that has one", () => {
    expect(opening({ platform: "linux", env: {} }, holding("gio", "xdg-open"))).toMatchObject({
      program: "gio",
      before: ["open"],
    });
  });

  it("is `xdg-open` where there is no `gio`", () => {
    expect(opening({ platform: "linux", env: {} }, holding("xdg-open"))).toMatchObject({
      program: "xdg-open",
      before: [],
    });
  });

  /// And still `xdg-open` where neither was found, which leaves that machine
  /// exactly where `shell` would have left it: `xdg-open` is the program `shell`
  /// runs, so a fallback to it is no loss.
  it("is `xdg-open` where neither was found", () => {
    expect(opening({ platform: "linux", env: {} }, holding())).toMatchObject({
      program: "xdg-open",
    });
  });

  /// The two platforms with no subclass to follow keep Electron's own, which is
  /// what `undefined` says to both callers.
  it("is `shell`'s own job on a Mac and on Windows", () => {
    for (const platform of ["darwin", "win32"] as const) {
      expect(opening({ platform, env: {} }, holding("gio")), platform).toBeUndefined();
    }
  });
});

describe("handing something over", () => {
  /// A stand-in opener, which records the whole of what it was given and exits.
  function standIn(): { program: string; record: string } {
    const where = mkdtempSync(join(tmpdir(), "verkstead-opening-"));
    const program = join(where, "stand-in-opener");
    const record = join(where, "record");

    writeFileSync(
      program,
      ["#!/bin/sh", `{ echo "$@"; echo "\${LD_LIBRARY_PATH-}"; } > '${record}'`, ""].join("\n"),
      { mode: 0o755 },
    );

    return { program, record };
  }

  /// Wait for the stand-in to have written both its lines — a redirect lands a
  /// line at a time, so a read that took the first for the whole record would
  /// see the loader as empty whatever the app did.
  async function recorded(record: string): Promise<string[]> {
    for (let asked = 0; asked < 200; asked += 1) {
      try {
        const said = readFileSync(record, "utf8").split("\n");
        // Two lines and the tail the last `echo` leaves, as in
        // `sidecar.test.ts`: a shorter read is a redirect caught mid-write.
        if (said.length > 2) {
          return said;
        }
      } catch {
        // Not written yet.
      }
      await new Promise((then) => setTimeout(then, 10));
    }
    throw new Error(`${record} never said what it was given`);
  }

  it("starts the program on what is being opened", async () => {
    const { program, record } = standIn();

    hand({ program, before: ["open"], env: {} }, "/tmp/verkstead.log", () => {});

    const [said] = await recorded(record);
    expect(said).toBe("open /tmp/verkstead.log");
  });

  /// The one refusal `shell` could never report: a program that is not there at
  /// all. Said into the app's own log, because the human has picked a menu item
  /// and nothing else is going to happen.
  it("says so where there is no such program", async () => {
    const lines: string[] = [];

    hand(
      { program: join(tmpdir(), "no-opener-of-this-name"), before: [], env: {} },
      "/tmp/verkstead.log",
      (line) => lines.push(line),
    );

    for (let asked = 0; asked < 100 && lines.length === 0; asked += 1) {
      await new Promise((then) => setTimeout(then, 20));
    }

    expect(lines.join("\n")).toContain("could not be handed to");
  });

  /// And the one `xdg-open` never reports and `gio` does: a type nothing on this
  /// desktop is registered for, which comes back as a non-zero exit.
  it("says so where the program would not open it", async () => {
    const { program } = standIn();
    writeFileSync(program, "#!/bin/sh\nexit 3\n", { mode: 0o755 });
    const lines: string[] = [];

    hand({ program, before: [], env: {} }, "/tmp/verkstead.log", (line) => lines.push(line));

    for (let asked = 0; asked < 100 && lines.length === 0; asked += 1) {
      await new Promise((then) => setTimeout(then, 20));
    }

    expect(lines.join("\n")).toContain("did not open /tmp/verkstead.log");
  });
});
