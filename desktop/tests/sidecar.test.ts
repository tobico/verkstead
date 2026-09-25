//! The sidecar's lifetime, proved against a real child process.
//!
//! A stand-in rather than the workspace's own `verkstead`: what these tests are
//! about is what the app does with a child — how it starts one, what it hands
//! it, and that it never leaves one behind — and a real server would put a
//! database and a port in the way of all three. The stand-in records what it
//! was started with and then either sits there or falls over, which between
//! them are the two lifetimes a sidecar has from here.

import { spawn } from "node:child_process";
import { chmodSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { ARGUMENTS, how, start, type Sidecar } from "../src/sidecar.js";

/// A stand-in `verkstead`, written somewhere of its own and made runnable.
///
/// Where it records what it was started with is written into the script rather
/// than handed to it: the app hands a sidecar nothing but the environment it
/// already had, so a test that needed a variable of its own would be testing a
/// door this has not got.
///
/// `exec`, so that the process the app is holding *is* the one that sits there
/// — a shell that forked and waited would be a shell the app signals and a
/// sleep it does not.
///
/// What it does after recording is the variable, because a server that sits
/// there and a server that falls over are the two lifetimes the app has to tell
/// apart.
function standIn(then = "exec sleep 600"): { cli: string; record: string } {
  const where = mkdtempSync(join(tmpdir(), "verkstead-sidecar-"));
  const cli = join(where, "verkstead");
  const record = join(where, "record");
  writeFileSync(
    cli,
    [
      "#!/bin/sh",
      `{ echo "$@"; echo "\${VERKSTEAD_DATA_DIR-}"; } > '${record}'`,
      then,
      "",
    ].join("\n"),
  );
  chmodSync(cli, 0o755);
  return { cli, record };
}

/// Wait until the stand-in has said what it was started with, which is the
/// first thing it does.
async function recorded(record: string): Promise<string[]> {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    try {
      const written = readFileSync(record, "utf8");
      if (written.split("\n").length >= 2) {
        return written.split("\n");
      }
    } catch {
      // Not written yet.
    }
    await new Promise((on) => setTimeout(on, 10));
  }
  throw new Error(`the stand-in never wrote ${record}`);
}

/// Whether a process is still there. A pid that has gone is `ESRCH`; one that
/// belongs to somebody else is `EPERM`, which is still a pid in use.
const alive = (pid: number): boolean => {
  try {
    process.kill(pid, 0);
    return true;
  } catch (trouble) {
    return (trouble as NodeJS.ErrnoException).code === "EPERM";
  }
};

/// Wait for a pid to go, up to a bound.
async function gone(pid: number): Promise<boolean> {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    if (!alive(pid)) {
      return true;
    }
    await new Promise((on) => setTimeout(on, 10));
  }
  return false;
}

let started: Sidecar | undefined;

afterEach(async () => {
  if (started) {
    started.stop();
    await started.gone;
    started = undefined;
  }
});

// A `/bin/sh` stand-in and a signalled process: this is what a sidecar is on
// the two platforms the app is developed on, and what it is on Windows is the
// packaging stages' question rather than this one's.
describe.skipIf(process.platform === "win32")("the sidecar", () => {
  it("is started as `serve --desktop` and nothing else", async () => {
    const { cli, record } = standIn();
    started = start(cli);

    const [said] = await recorded(record);
    expect(said).toBe(ARGUMENTS.join(" "));
  });

  /// Which is how a checkout run reaches the checkout's data: the app grows no
  /// flag of its own, and `VERKSTEAD_DATA_DIR` in the shell that started it is
  /// the server's own variable arriving where the server reads it.
  it("inherits the environment it was started in", async () => {
    const { cli, record } = standIn();
    process.env.VERKSTEAD_DATA_DIR = "/srv/somewhere";
    try {
      started = start(cli);
      const [, data] = await recorded(record);
      expect(data).toBe("/srv/somewhere");
    } finally {
      delete process.env.VERKSTEAD_DATA_DIR;
    }
  });

  it("goes when it is stopped", async () => {
    const { cli, record } = standIn();
    const sidecar = start(cli);
    await recorded(record);

    sidecar.stop();
    await sidecar.gone;

    expect(alive(sidecar.pid!)).toBe(false);
  });

  /// Because the server ending is the app quitting, and the line saying so is
  /// the whole account anybody gets of why: a server that fell over on its own
  /// reads differently from one the app asked to stop, and the difference is
  /// what the child handed over as it went.
  it("says a server that ended on its own exited", async () => {
    const { cli, record } = standIn("exit 3");
    started = start(cli);
    await recorded(record);

    const ending = await started.gone;

    expect(ending).toEqual({ code: 3, signal: null });
    expect(how(ending)).toContain("status 3");
  });

  it("says a server that was stopped was killed", async () => {
    const { cli, record } = standIn();
    started = start(cli);
    await recorded(record);

    started.stop();
    const ending = await started.gone;

    expect(ending).toEqual({ code: null, signal: "SIGTERM" });
    expect(how(ending)).toContain("SIGTERM");
  });

  /// And the path nobody chose: the app ending without a quit — killed, or
  /// falling over — is still an app that must not leave a server holding the
  /// one port the next launch needs. Asked of a process of its own, because
  /// there is no other way to end this one and go on asserting.
  it("goes when the app ends without stopping it", async () => {
    const { cli } = standIn();
    const orphan = spawn(process.execPath, [join(import.meta.dirname, "fixtures", "orphan.mjs"), cli], {
      stdio: ["ignore", "pipe", "ignore"],
    });

    let said = "";
    orphan.stdout.on("data", (chunk: Buffer) => (said += chunk.toString()));
    await new Promise<void>((ended) => orphan.once("exit", () => ended()));

    const pid = Number(said.trim());
    expect(pid).toBeGreaterThan(0);
    expect(await gone(pid)).toBe(true);
  });
});

/// The two endings no stand-in can be made to have: a clean stop, which a
/// server asked to go has, and a child that handed over neither a status nor a
/// signal, which Node's own types leave room for.
describe("an ending in words", () => {
  it("has a clean exit read as one", () => {
    expect(how({ code: 0, signal: null })).toBe("it exited cleanly");
  });

  it("says so rather than nothing where there is nothing to say", () => {
    expect(how({ code: null, signal: null })).toBe("it ended without saying how");
  });
});
