//! Handing a file or a URL to whatever this desktop opens that kind of thing
//! with — which on Linux is the app's own doing and on the other two is
//! Electron's.
//!
//! Two things are ever handed over, and to this app they are one act: the log
//! file, which goes to whatever reads text here, and a link that leaves the
//! workbench, which goes to a browser. `crates/desktop/src/opener.rs` was one
//! module for the same pair and for the same reason — what the desktop starts
//! is the desktop's business rather than Verkstead's.
//!
//! **Linux is the arm with anything in it, and `xdg-open` is why.**
//! `shell.openPath` and `shell.openExternal` are that program, and it does not
//! follow a mime type's subclass. This run's log file is `text/x-log`, so on a
//! desktop with an association for `text/plain` and none for that subtype
//! nothing opens: stage 03 of the Electron roadmap measured it **exiting 0**
//! having done so, which leaves the app reporting success, and a machine with no
//! handler for the subtype at all refuses outright. Either way **View Logs** is
//! a button that does nothing — on the desktop most likely to need it, the log
//! being where the app says what went wrong. `gio open` follows the subclass, so
//! it is asked first where the machine has one, and `xdg-open` is what is left.
//!
//! **And whichever of them it is, it is started with
//! [`unmounted`](./unmounted.js)'s environment.** A packed run's `AppRun` leads
//! `LD_LIBRARY_PATH` with the bundle's own `usr/lib` — `libXss`, `libXtst`,
//! `libnotify`, `libappindicator3`, `libindicator3` and `libgconf-2`, in the
//! image electron-builder packs — and a child of `shell` inherits it, so the
//! human's own editor and their own browser would load this bundle's copies of
//! libraries they link themselves. electron-builder's own `AppRun` guards its
//! dialogs with an emptied `LD_LIBRARY_PATH` for exactly that. The sidecar is
//! handed the same answer at the same door — see `main.ts`, which is where the
//! other one is — and this is the other half of one question rather than a
//! second one.
//!
//! **The other platforms keep `shell`.** There is no mount and no subclass to
//! follow on a Mac
//! or on Windows — a `.log` there is opened by whatever the platform associates
//! with the extension — so [`opening`] says nothing about them and the edge
//! file's own `shell` call is what runs.
//!
//! Nothing here reaches the running application, so vitest runs all of it: the
//! choice is a function of the environment and of what is on the `PATH`, and the
//! handing over is a child process like the sidecar's.

import { spawn } from "node:child_process";
import { statSync } from "node:fs";
import { join } from "node:path";

import type { Machine } from "./platform.js";
import { type Environment, unmounted } from "./unmounted.js";

/// `gio open <what>`, which follows the subclass — GLib's own opener, and there
/// wherever GLib is, which on a desktop is everywhere.
const GIO = ["gio", "open"] as const;

/// And `xdg-open <what>`, which does not. Last because it is the one `shell`
/// would have run anyway, so a machine with no `gio` is left exactly where it
/// was rather than somewhere worse.
const XDG = ["xdg-open"] as const;

/// The openers, most preferred first.
const OPENERS = [GIO, XDG] as const;

/// How this machine opens something of the app's.
export interface Opening {
  /// The program to start.
  readonly program: string;

  /// What goes in front of the file or the URL on its command line.
  readonly before: readonly string[];

  /// And the environment it is started with, which is the one the sidecar is
  /// given: this process's own, with the AppImage's doing out of it.
  readonly env: Environment;
}

/// Whether `program` is on `path`.
///
/// A plain lookup rather than a shell's: the entries of a `PATH` in order, and
/// any of them holding a file of that name. Empty entries are the current
/// directory to a shell and nothing to this — a program found beside wherever
/// the app happens to be standing is not one the human asked for.
function onThePath(path: string | undefined, program: string): boolean {
  return (path ?? "")
    .split(":")
    .filter((entry) => entry !== "")
    .some((entry) => statSync(join(entry, program), { throwIfNoEntry: false })?.isFile() === true);
}

/// How this machine opens a file or a URL, and `undefined` where opening it is
/// `shell`'s job — which is every platform but Linux.
///
/// `there` is the look on the `PATH`, taken as an argument so that both arms of
/// the choice are arms a test calls. The `PATH` it is looked along is the
/// child's own, with the mount already out of it: a `gio` inside the bundle
/// would be one this app found and the machine could not keep, the mount going
/// when the app does.
export function opening(
  machine: Machine,
  there: (path: string | undefined, program: string) => boolean = onThePath,
): Opening | undefined {
  if (machine.platform !== "linux") {
    return undefined;
  }

  const env = unmounted(machine.env);
  const [program, ...before] = OPENERS.find(([named]) => there(env.PATH, named)) ?? XDG;

  return { program, before, env };
}

/// Hand `what` over, and say so where it could not be.
///
/// **Not waited on**, the same reading `opener.rs` made of it: what starts is
/// somebody else's program, and an editor that takes ten seconds to come up is
/// not something the app should be sitting on. Detached and unreferenced with
/// it, so that what the human opened outlives the Verkstead that opened it.
///
/// Both endings are read, because a refusal here is a refusal that reaches
/// nobody otherwise: `gio` says on a non-zero exit that nothing is registered
/// for the type, and a program that is not there at all never runs to say
/// anything.
export function hand(by: Opening, what: string, said: (line: string) => void): void {
  const handing = spawn(by.program, [...by.before, what], {
    env: by.env,
    stdio: "ignore",
    detached: true,
  });

  handing.on("error", (trouble: unknown) => {
    said(`${what} could not be handed to ${by.program} — ${String(trouble)}`);
  });

  handing.on("exit", (code, signal) => {
    if (code !== 0) {
      said(`${by.program} did not open ${what} — it ended with ${signal ?? code}`);
    }
  });

  handing.unref();
}
