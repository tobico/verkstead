//! The wait for the server, asked of a real server.
//!
//! A stub over `fetch` would prove the loop against itself; what is worth
//! proving is what a starting Verkstead actually looks like from here — a
//! refused connection, then a listener that is up but not serving, then `ok` —
//! so the test stands an HTTP server of its own and moves it through the three.

import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import { healthy, NeverCameUp } from "../src/health.js";

/// The waiting these tests do, which is nobody's half a minute.
const BRISKLY = { patience: 2_000, every: 10 };

/// A server that answers whatever the current `answer` says, on a port the
/// machine picked.
function standing(answer: () => number): Promise<{ url: string; server: Server }> {
  const server = createServer((_, response) => {
    response.statusCode = answer();
    response.end("ok");
  });
  return new Promise((up) => {
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address() as AddressInfo;
      up({ url: `http://127.0.0.1:${port}/api/v1/health`, server });
    });
  });
}

/// A port nothing is listening on, which is what a sidecar that has not bound
/// yet looks like. Taken by standing a server up and shutting it down again, so
/// that the number is one the machine agreed was free.
async function nobody(): Promise<string> {
  const { url, server } = await standing(() => 200);
  await new Promise<void>((shut) => server.close(() => shut()));
  return url;
}

let standing_up: Server | undefined;

afterEach(async () => {
  if (standing_up) {
    await new Promise<void>((shut) => standing_up!.close(() => shut()));
    standing_up = undefined;
  }
});

describe("waiting for the server", () => {
  it("says how long a server that is already up took", async () => {
    const { url, server } = await standing(() => 200);
    standing_up = server;

    await expect(healthy(url, BRISKLY)).resolves.toBeTypeOf("number");
  });

  /// A listener that is up but not finished is not an answer: the server binds
  /// and then goes on deciding what this machine can do, and a 503 in the
  /// middle of that is the thing being waited for rather than a failure.
  it("goes on asking a server that is up but not serving", async () => {
    let status = 503;
    const { url, server } = await standing(() => status);
    standing_up = server;

    const waited = healthy(url, BRISKLY);
    setTimeout(() => (status = 200), 50);

    await expect(waited).resolves.toBeGreaterThanOrEqual(50);
  });

  /// And the bound, which is what a developer waiting on nothing gets told
  /// instead of nothing.
  it("gives up at the bound, naming what it was asking", async () => {
    const url = await nobody();

    await expect(healthy(url, { patience: 150, every: 10 })).rejects.toThrow(NeverCameUp);
    await expect(healthy(url, { patience: 150, every: 10 })).rejects.toThrow(url);
  });
});
