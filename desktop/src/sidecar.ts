//! The server, started beside the app and owned by it.
//!
//! **`serve --desktop`, and the address.** The flag is the whole of what the
//! server is told about who started it (ADR-0020), and the address is the one
//! thing the app has an opinion about: the port is fixed by decision and the
//! app probes it, waits on it and loads it, so the server is *told* to bind it
//! rather than left to resolve it — the child gets the environment the app was
//! started in, and a `VERKSTEAD_LISTEN` exported in the shell that started the
//! app would otherwise put the server somewhere the rest of the app is not
//! looking. See [`ADDRESS`](./workbench.js), which is the number, and
//! [`LISTEN`](./workbench.js), which the app reports having overridden.
//!
//! **Every other setting is still the server's own.** A checkout run reaching
//! the checkout's data is `VERKSTEAD_DATA_DIR` in the shell that started the
//! app, exactly as it is for a `verkstead serve` run by hand, and a packed app
//! that says nothing gets the platform **Data Directory**. The app grows no
//! flag for any of it.
//!
//! **The environment is handed in rather than inherited**, which is what lets
//! the app take the AppImage's own doing out of it first — see
//! [`unmounted`](./unmounted.js), which is what `main.ts` passes through. What
//! a child gets here is what it was given and nothing this module resolved.
//!
//! **And the child dies with the app.** Not as a courtesy: the port is fixed,
//! so a sidecar left running is the next launch meeting a foreign listener on
//! its own address. Two mechanisms, because the ways out of a process are not
//! one thing — the app quitting is a [`stop`](Sidecar.stop) the caller makes,
//! and the process ending for a reason nobody handled is the `exit` hook below,
//! which is the last thing Node runs and takes the child with it.
//!
//! **Two gaps, and only one of them is nobody's fault.** `SIGKILL` on the app
//! itself no process can answer for; a terminal's Ctrl-C is not that, being sent
//! to the whole process group and so to the child as well. The other is
//! Electron's own `app.exit`, which ends the process without running the `exit`
//! hook — measured, not assumed — so every path in `main.ts` that takes it
//! calls [`stop`](Sidecar.stop) first, and the hook is what covers the ways out
//! nobody wrote down.

import { spawn, type ChildProcess } from "node:child_process";
import type { Readable } from "node:stream";

/// What the sidecar is started with, for the address the app serves on.
/// Everything else is the server's own.
///
/// A function of the address rather than a constant holding it, because this
/// module imports nothing — the fixture in `tests/fixtures/` runs it straight
/// off the TypeScript, which Node reads by stripping the types and so cannot
/// follow a `.js` import out of. The one caller that knows the number is the
/// one that already knows everything else about the running application.
export const ARGUMENTS = (address: string): string[] => [
  "serve",
  "--desktop",
  "--listen",
  address,
];

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

/// A stream being read a line at a time.
export interface Lines {
  /// Whatever the pipe just handed over.
  feed(chunk: string): void;
  /// The stream ended: what is left is a last line that carried no newline.
  end(): void;
}

/// Read whole lines out of a stream that arrives in chunks.
///
/// A pipe hands over bytes as they arrive rather than events as they were
/// logged, and one `tracing` line can cross two chunks as easily as two lines
/// can share one. The log file rolls between lines, so lines are what it has to
/// be handed.
export function byLine(onLine: (line: string) => void): Lines {
  let rest = "";

  const said = (line: string): void => {
    // A Windows sidecar's own line endings are not the log file's.
    onLine(line.endsWith("\r") ? line.slice(0, -1) : line);
  };

  return {
    feed(chunk) {
      rest += chunk;

      for (let end = rest.indexOf("\n"); end >= 0; end = rest.indexOf("\n")) {
        said(rest.slice(0, end));
        rest = rest.slice(end + 1);
      }
    },

    end() {
      if (rest === "") {
        return;
      }

      const last = rest;
      rest = "";
      said(last);
    },
  };
}

/// Read one of the child's streams to `said`, a line at a time.
///
/// Decoded by the stream rather than by the reader, so a character split across
/// two chunks arrives as one character rather than as two broken ones —
/// Verkstead's own messages have em-dashes in them, which is the same reason
/// the log file carries a byte-order mark.
function read(stream: Readable | null, said: (line: string) => void): void {
  if (stream === null) {
    return;
  }

  const lines = byLine(said);

  stream.setEncoding("utf8");
  stream.on("data", (chunk: string) => lines.feed(chunk));

  // What a sidecar taken mid-line had said: the stream ends without a newline
  // and the last thing it said is the account of why the app is going with it.
  stream.once("end", () => lines.end());
}

/// Start the server beside this app.
///
/// The binary is [`cli`](./cli.js)'s answer, and whether anything is at that
/// path is the caller's question — a spawn of a path with nothing at it fails
/// asynchronously, which is a dialog nobody can word.
///
/// **`address` is what the server is told to bind**, for the reason at the top
/// of this file: the app has one address and it is the app that names it.
///
/// **`said` is every line the server logs**, handed over as it says it. The app
/// passes [`heard`](./log.js), which puts those lines in `verkstead.log` beside
/// its own — taken as an argument rather than imported so that this module
/// answers to nothing but a child process, which is what lets the fixture in
/// `tests/fixtures/` run it straight off the TypeScript.
///
/// **`env` is the child's whole environment**, for the reason at the top of this
/// file: the app passes [`unmounted`](./unmounted.js) of its own, so that a
/// packed run does not pass the bundle's directories on to a server and to every
/// session under it.
export function start(
  cli: string,
  address: string,
  said: (line: string) => void,
  env: Partial<Record<string, string>>,
): Sidecar {
  const child = spawn(cli, ARGUMENTS(address), {
    // Nothing on stdin, and both streams read rather than inherited: the log
    // file is the app's to write, so the sidecar's `tracing` output comes
    // through this process and lands in `verkstead.log` beside the app's own
    // lines. Both of them, because what a server says as it falls over goes to
    // stderr and that is the half most worth having in the file.
    stdio: ["ignore", "pipe", "pipe"],
    env,
  });

  read(child.stdout, said);
  read(child.stderr, said);

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
