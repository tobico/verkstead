//! The probe, asked of a real listener.
//!
//! The question it answers decides whether the app starts anything at all, and
//! the two answers are two states of the machine rather than two branches of
//! the code — so the test puts the machine in each. A server of its own on a
//! port the machine picked: the address is somebody's, and then, once it has
//! been shut down, nobody's.

import { createServer, type Server } from "node:net";
import type { AddressInfo } from "node:net";

import { afterEach, describe, expect, it } from "vitest";

import { taken } from "../src/taken.js";

/// Loopback, which is the only interface any of this is about.
const HERE = "127.0.0.1";

let listening: Server | undefined;

/// A listener that accepts and says nothing — which is all the probe asks of
/// it, and which is also what a server mid-startup looks like from here.
function stand(): Promise<number> {
  const server = createServer();
  listening = server;
  return new Promise((up) => {
    server.listen(0, HERE, () => up((server.address() as AddressInfo).port));
  });
}

/// Take it down again, and hand back the port it had. Nothing else is listening
/// there by the time this returns, so the number is an address that is free.
async function down(): Promise<void> {
  const server = listening;
  listening = undefined;
  if (server !== undefined) {
    await new Promise<void>((shut) => server.close(() => shut()));
  }
}

afterEach(down);

describe("the probe", () => {
  it("says an address somebody is listening on is taken", async () => {
    const port = await stand();

    await expect(taken(HERE, port)).resolves.toBe(true);
  });

  it("says an address nobody answers on is free", async () => {
    const port = await stand();
    await down();

    await expect(taken(HERE, port)).resolves.toBe(false);
  });

  /// Which is the whole run, in the order it happens: the app asks, and then
  /// the sidecar binds the address it asked about. A probe that answered by
  /// taking the address would be the thing standing in the way of the start it
  /// was clearing.
  it("leaves a free address free to be bound", async () => {
    const port = await stand();
    await down();
    expect(await taken(HERE, port)).toBe(false);

    const sidecar = createServer();
    listening = sidecar;
    await new Promise<void>((bound, refused) => {
      sidecar.once("error", refused);
      sidecar.listen(port, HERE, () => bound());
    });

    await expect(taken(HERE, port)).resolves.toBe(true);
  });

  /// A refusal is what loopback gives at once, so the patience is never spent
  /// on the answer that matters. Worth one assertion because the bound is a
  /// second and the app waits on it before it starts anything.
  it("does not spend its patience on a refusal", async () => {
    const port = await stand();
    await down();

    const started = Date.now();
    await taken(HERE, port, 5_000);

    expect(Date.now() - started).toBeLessThan(1_000);
  });
});
