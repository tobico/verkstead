//! Waiting for the sidecar to be a server.
//!
//! Starting the process is not the same as being able to serve: the server opens
//! its database, runs its migrations and decides what this machine can do before
//! it binds, and until it has there is nothing at the port at all. So the app
//! asks the one route outside the gate until it answers — see
//! [`HEALTH`](./workbench.js) for why that route and no other.
//!
//! **A bound, and it is worth reporting.** A wait with no end is an app that
//! sits there saying nothing while a developer wonders which half is wrong, so
//! the poll gives up and the caller says which of the two happened. The bound is
//! generous because the thing being waited on is slowest exactly once — a first
//! start, on a cold database, on a machine that has just decompressed the app.

/// How long to wait before saying the server never came up.
const PATIENCE = 30_000;

/// How long between asks. Short, because the common case is a server that is
/// already there and the cost of asking is a refused connection on loopback.
const EVERY = 100;

/// What a caller may say about the waiting instead. Both are for the tests,
/// which have no interest in half a minute of patience.
export interface Waiting {
  /// The bound, in milliseconds.
  patience?: number;
  /// The gap between asks, in milliseconds.
  every?: number;
}

/// What the wait comes to: how long it took, or why it stopped.
export class NeverCameUp extends Error {
  constructor(
    readonly url: string,
    readonly waited: number,
  ) {
    super(`${url} did not answer within ${waited} ms`);
    this.name = "NeverCameUp";
  }
}

/// Sleep, which is the whole of what happens between two asks.
const pause = (ms: number) => new Promise<void>((done) => setTimeout(done, ms));

/// Ask until the server answers, and say how long that took.
///
/// Throws [`NeverCameUp`] at the bound. Every other outcome is another ask:
/// a refused connection is a server that has not bound yet, and a status that
/// is not a success is a server that is up but not finished — both are the
/// thing this is waiting for rather than something to report.
export async function healthy(url: string, waiting: Waiting = {}): Promise<number> {
  const patience = waiting.patience ?? PATIENCE;
  const every = waiting.every ?? EVERY;
  const started = Date.now();

  for (;;) {
    const waited = () => Date.now() - started;

    try {
      // A timeout of its own, because a connection that is accepted and then
      // never answered would otherwise hold the whole bound in one ask.
      const answer = await fetch(url, { signal: AbortSignal.timeout(every + 1_000) });
      if (answer.ok) {
        return waited();
      }
    } catch {
      // Refused, reset, timed out: a server that is not there yet says all
      // three, and none of them is different from the others here.
    }

    if (waited() >= patience) {
      throw new NeverCameUp(url, waited());
    }

    await pause(every);
  }
}
