//! The server, started beside the app and owned by it.
//!
//! **`serve --desktop` and nothing else.** The flag is the whole of what the
//! server is told about who started it (ADR-0020) and every other setting is
//! the server's own: the child inherits this process's environment, so a
//! checkout run reaching the checkout's data is `VERKSTEAD_DATA_DIR` in the
//! shell that started the app, exactly as it is for a `verkstead serve` run by
//! hand, and a packed app that says nothing gets the platform **Data
//! Directory**. The app grows no flag for any of it.
//!
//! **And the child dies with the app.** Not as a courtesy: the port is fixed,
//! so a sidecar left running is the next launch meeting a foreign listener on
//! its own address. Two mechanisms, because the ways out of a process are not
//! one thing — the app quitting is a [`stop`](Sidecar.stop) the caller makes,
//! and the process ending for any other reason is the `exit` hook below, which
//! is the last thing Node runs and takes the child with it. The one gap is
//! `SIGKILL` on the app itself, which no process can answer for; a terminal's
//! Ctrl-C is not that, being sent to the whole process group and so to the
//! child as well.

import { spawn, type ChildProcess } from "node:child_process";

/// What the sidecar is started with. Everything else is the server's own.
export const ARGUMENTS = ["serve", "--desktop"];

/// How long a stopped sidecar is given to go before it is taken.
const GRACE = 5_000;

/// How a child left: the status it gave, or the signal that took it. Exactly
/// one of the two, which is what Node's own `exit` hands over.
export interface Ending {
  /// What it exited with, where it exited of its own accord.
  readonly code: number | null;
  /// What took it, where something did.
  readonly signal: NodeJS.Signals | null;
}

/// An ending in the words the log says it in.
///
/// The server ending is the app quitting, so this line is the whole of the
/// account anybody gets of why the app went — a crash, a `kill`, or a clean
/// stop all end the same way from here and read differently only because of
/// this.
export function how({ code, signal }: Ending): string {
  if (signal !== null) {
    return `it was killed by ${signal}`;
  }
  if (code === 0) {
    return "it exited cleanly";
  }
  if (code === null) {
    return "it ended without saying how";
  }
  return `it exited with status ${code}`;
}

/// A running sidecar, and the two things the app does with one.
export interface Sidecar {
  /// The child's process id, for the app's own logging.
  readonly pid: number | undefined;
  /// Resolves when the child has gone, with how it went.
  readonly gone: Promise<Ending>;
  /// Ask it to stop, and take it if it will not.
  stop(): void;
}

/// Whether the child is still there to be signalled.
const running = (child: ChildProcess) => child.exitCode === null && child.signalCode === null;

/// Start the server beside this app.
///
/// The binary is [`cli`](./cli.js)'s answer, and whether anything is at that
/// path is the caller's question — a spawn of a path with nothing at it fails
/// asynchronously, which is a dialog nobody can word.
export function start(cli: string): Sidecar {
  const child = spawn(cli, ARGUMENTS, {
    // Nothing on stdin, and both streams through to this process's own: the
    // sidecar's `tracing` output lands beside the app's lines wherever the app
    // was started from. The log file that takes both is a later task's.
    stdio: ["ignore", "inherit", "inherit"],
  });

  const gone = new Promise<Ending>((ended) => {
    child.once("exit", (code, signal) => ended({ code, signal }));
  });

  // The last thing this process does, whatever ended it: a quit, an uncaught
  // error, an `app.exit` from somewhere else. There is no waiting to be done
  // in an `exit` handler, so this is the taking rather than the asking.
  process.once("exit", () => {
    if (running(child)) {
      child.kill("SIGKILL");
    }
  });

  return {
    pid: child.pid,
    gone,
    stop() {
      if (!running(child)) {
        return;
      }
      child.kill("SIGTERM");
      // And taken if it will not go. Unreferenced so that a sidecar which
      // stops promptly — which is all of them — does not hold the loop open
      // for five seconds after the app has finished with it.
      setTimeout(() => {
        if (running(child)) {
          child.kill("SIGKILL");
        }
      }, GRACE).unref();
    },
  };
}
