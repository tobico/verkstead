//! This device's Update, driven against a fake electron-updater.
//!
//! The real one is proven by `scripts/update-feed.mjs`, against a packed app and
//! a local feed; what is worth proving here is the exchange around it — the
//! order the updater is driven in, what the server is told and when, that
//! nothing is handed over on a check or a download that came to nothing, and
//! the wait a relaunch makes for the app it replaces.

import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  FEED,
  missed,
  prepare,
  RELAUNCH,
  remember,
  told,
  update,
  type Updater,
  waitOut,
} from "../src/updating.js";

/// An updater that says `version` is the latest — or nothing newer, where it
/// is `null` — and downloads through `steps`, writing down what it was asked.
function faked(version: string | null, steps: number[], failing?: string) {
  const asked: string[] = [];
  let progress: ((progress: { percent: number }) => void) | undefined;

  const updater: Updater = {
    async checkForUpdates() {
      asked.push("check");
      return version === null
        ? { isUpdateAvailable: false, updateInfo: { version: "0.2.2" } }
        : { isUpdateAvailable: true, updateInfo: { version } };
    },
    async downloadUpdate() {
      asked.push("download");
      for (const percent of steps) {
        progress?.({ percent });
      }
      if (failing !== undefined) {
        throw new Error(failing);
      }
      return [];
    },
    quitAndInstall(isSilent, isForceRunAfter) {
      asked.push(`install ${isSilent} ${isForceRunAfter}`);
    },
    on(_event, heard) {
      progress = heard;
      return this;
    },
  };

  return { updater, asked };
}

/// The exchange around `updater`, writing down what the server was told and
/// when the app handed over.
function around(updater: Updater) {
  const reported: number[] = [];
  const failed: string[] = [];
  const done: string[] = [];

  return {
    reported,
    failed,
    done,
    updating: {
      updater,
      report: async (percent: number) => {
        reported.push(percent);
      },
      fail: async (why: string) => {
        failed.push(why);
      },
      handOver: (version: string) => {
        done.push(`handed over to ${version}`);
      },
      say: () => {},
    },
  };
}

describe("an Update", () => {
  /// Check, download, then hand over and `quitAndInstall`, silent and
  /// relaunching — and the handing over before the install, which is what
  /// makes the sidecar's going expected rather than news.
  it("is checked, downloaded, then handed over", async () => {
    const { updater, asked } = faked("0.3.0", [12.5, 12.9, 50, 99.9]);
    const exchange = around(updater);
    const order: string[] = [];
    exchange.updating.handOver = (version) => {
      order.push(`handed over to ${version} after ${asked.length}`);
    };

    const outcome = await update(exchange.updating);

    expect(outcome).toEqual({ kind: "handed-over", version: "0.3.0" });
    expect(asked).toEqual(["check", "download", "install true true"]);
    expect(order).toEqual(["handed over to 0.3.0 after 2"]);
    expect(exchange.failed).toEqual([]);
  });

  /// The server is told a whole percent at a time, each once, from nought to
  /// a hundred.
  it("tells the server each whole percent once", async () => {
    const { updater } = faked("0.3.0", [0.4, 12.5, 12.9, 50, 99.9]);
    const exchange = around(updater);

    await update(exchange.updating);

    expect(exchange.reported).toEqual([0, 12, 50, 99, 100]);
  });

  /// Nothing newer is nothing downloaded and nothing handed over — and told
  /// to the server, whose row would otherwise read *updating…* on.
  it("downloads nothing where there is nothing newer", async () => {
    const { updater, asked } = faked(null, []);
    const exchange = around(updater);

    expect(await update(exchange.updating)).toEqual({ kind: "nothing-newer" });
    expect(asked).toEqual(["check"]);
    expect(exchange.done).toEqual([]);
    expect(exchange.failed).toEqual(["There's no newer release to update to."]);
  });

  /// A download that fails hands nothing over, and tells the server the
  /// updater's own reason.
  it("hands nothing over when the download fails", async () => {
    const { updater, asked } = faked("0.3.0", [20], "sha512 checksum mismatch");
    const exchange = around(updater);

    expect(await update(exchange.updating)).toEqual({
      kind: "failed",
      why: "sha512 checksum mismatch",
    });
    expect(asked).toEqual(["check", "download"]);
    expect(exchange.done).toEqual([]);
    expect(exchange.failed).toEqual(["sha512 checksum mismatch"]);
  });

  /// A server that can't be told of the failure is said, not thrown.
  it("comes out failed when the failure can't be told", async () => {
    const { updater } = faked("0.3.0", [], "net::ERR_CONNECTION_REFUSED");
    const exchange = around(updater);
    exchange.updating.fail = () => Promise.reject(new Error("refused"));

    expect((await update(exchange.updating)).kind).toBe("failed");
  });

  /// A server that refuses a report costs the report, not the Update.
  it("carries on past a report the server refused", async () => {
    const { updater, asked } = faked("0.3.0", [50]);
    const exchange = around(updater);
    exchange.updating.report = () => Promise.reject(new Error("refused"));

    expect((await update(exchange.updating)).kind).toBe("handed-over");
    expect(asked.at(-1)).toBe("install true true");
  });
});

describe("the updater", () => {
  /// Set up to download and install only when asked, and read from the
  /// test's feed only where one is named.
  it("is pointed at a local feed only where the test says one", () => {
    const fed: unknown[] = [];
    const updater = () => ({
      ...faked("0.3.0", []).updater,
      autoDownload: true,
      autoInstallOnAppQuit: true,
      logger: null as unknown,
      setFeedURL: (options: unknown) => fed.push(options),
    });

    const left = updater();
    prepare(left, {}, () => {});
    expect(left.autoDownload).toBe(false);
    expect(left.autoInstallOnAppQuit).toBe(false);
    expect(fed).toEqual([]);

    prepare(updater(), { [FEED]: "http://127.0.0.1:9/feed" }, () => {});
    expect(fed).toEqual([{ provider: "generic", url: "http://127.0.0.1:9/feed" }]);
  });
});

describe("a report", () => {
  /// Posted with the key as the gate's cookie and the percent as the body.
  it("is posted with the key", async () => {
    const heard: { cookie?: string; body: string; method?: string }[] = [];
    const server = createServer((request, response) => {
      let body = "";
      request.on("data", (chunk) => (body += chunk));
      request.on("end", () => {
        heard.push({ cookie: request.headers.cookie, body, method: request.method });
        response.statusCode = 204;
        response.end();
      });
    });
    await new Promise<void>((up) => server.listen(0, "127.0.0.1", () => up()));
    const { port } = server.address() as AddressInfo;

    try {
      await told(`http://127.0.0.1:${port}/progress`, "the-key", { percent: 42 });
      await told(`http://127.0.0.1:${port}/failed`, "the-key", { why: "sha512 mismatch" });
    } finally {
      server.close();
    }

    expect(heard).toEqual([
      { cookie: "workbench_key=the-key", body: '{"percent":42}', method: "POST" },
      { cookie: "workbench_key=the-key", body: '{"why":"sha512 mismatch"}', method: "POST" },
    ]);
  });
});

describe("the version handed over to", () => {
  const scratch = () => join(mkdtempSync(join(tmpdir(), "handed-")), "updating.json");

  /// A start on the version remembered is the Update landed, and forgets it.
  it("is forgotten by a start on it", () => {
    const file = scratch();
    remember(file, "0.3.0", () => {});

    expect(missed(file, "0.3.0")).toBeUndefined();
    expect(existsSync(file)).toBe(false);
  });

  /// A start on another one is the Update that didn't land, said once.
  it("is the trouble line for a start on another version", () => {
    const file = scratch();
    remember(file, "0.3.0", () => {});

    expect(missed(file, "0.2.2")).toBe(
      "Verkstead 0.3.0 was installed, but this device started on 0.2.2.",
    );
    expect(existsSync(file)).toBe(false);
    expect(missed(file, "0.2.2")).toBeUndefined();
  });

  /// Nothing remembered, or nothing readable, is nothing to say.
  it("says nothing where nothing readable was remembered", () => {
    const file = scratch();
    expect(missed(file, "0.2.2")).toBeUndefined();

    writeFileSync(file, "not json");
    expect(missed(file, "0.2.2")).toBeUndefined();
    expect(existsSync(file)).toBe(false);
  });

  /// A file that can't be written is said, and the Update goes on.
  it("is said rather than thrown where it can't be kept", () => {
    const said: string[] = [];
    const dir = mkdtempSync(join(tmpdir(), "handed-"));
    rmSync(dir, { recursive: true });

    remember(join(dir, "updating.json"), "0.3.0", (line) => said.push(line));

    expect(said).toHaveLength(1);
  });
});

describe("a relaunch", () => {
  const BRISKLY = { patience: 2_000, every: 5 };

  /// A launch that replaces nothing waits for nothing.
  it("waits for nothing on an ordinary launch", async () => {
    const waited = await waitOut({}, {
      alive: () => true,
      taken: async () => true,
      ...BRISKLY,
    });

    expect(waited).toBeUndefined();
  });

  /// One that replaces an app waits until its pid has gone and then until
  /// the address is free, in that order.
  it("waits for the old app to go and then for its address", async () => {
    const asked: string[] = [];
    let lives = 3;
    let held = 2;

    const waited = await waitOut({ [RELAUNCH]: "424242" }, {
      alive: (pid) => {
        asked.push(`alive ${pid}`);
        return lives-- > 0;
      },
      taken: async () => {
        asked.push("taken");
        return held-- > 0;
      },
      ...BRISKLY,
    });

    expect(waited).toBeTypeOf("number");
    expect(asked).toEqual([
      "alive 424242",
      "alive 424242",
      "alive 424242",
      "alive 424242",
      "taken",
      "taken",
      "taken",
    ]);
  });

  /// And goes on at the bound, rather than waiting for ever on a pid that
  /// never goes.
  it("goes on at the bound", async () => {
    const waited = await waitOut({ [RELAUNCH]: "424242" }, {
      alive: () => true,
      taken: async () => false,
      patience: 50,
      every: 5,
    });

    expect(waited).toBeGreaterThanOrEqual(50);
  });
});
