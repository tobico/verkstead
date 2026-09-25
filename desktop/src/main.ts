//! The app itself: the one file that reads the running application.
//!
//! What it does at this stage is find the headless `verkstead`, start it as
//! `serve --desktop`, wait for the server to answer, and open one window on the
//! workbench logged in — owning the child's lifetime through all of it. Every
//! question it asks is answered by a module beside it that vitest can run
//! without an Electron, and the reads of the running application it makes for
//! itself are `app` and the process's own environment — see the wall in
//! `eslint.config.js`, and `window.ts`, which is the other file on it.
//!
//! **The order at the top of [`run`] is the lifecycle**, and it is an order
//! rather than a sequence of conveniences: the log file, so that every line
//! below it is in the file somebody will be asked to send; then the lock, so
//! that a second launch is the first window brought forward and never a second
//! sidecar; then the address, which the lock is what makes an unambiguous
//! question; then the binary; and only then a child. Everything before the
//! child is an app that can refuse having made nothing at all.

import { existsSync } from "node:fs";

import { app, dialog, type BrowserWindow } from "electron";

import { cli, OVERRIDE } from "./cli.js";
import { healthy, NeverCameUp } from "./health.js";
import { keyIn } from "./key.js";
import { heard, keep, say } from "./log.js";
import { dataDir, logDir } from "./platform.js";
import { how, start } from "./sidecar.js";
import { taken } from "./taken.js";
import { forward, open } from "./window.js";
import { ADDRESS, HEALTH, HOST, ORIGIN, PORT } from "./workbench.js";

/// What the human is told when the CLI is not where the app looked.
///
/// The path is the whole of what they need: a developer who has not run cargo
/// yet is the common case, and one running against a binary somewhere else is
/// the variable. Said in a dialog rather than on stdout because an app started
/// from an icon has no terminal for a line to land in.
function missing(path: string): void {
  dialog.showErrorBox(
    "Verkstead is not built",
    `The desktop app starts the headless verkstead beside itself, and there ` +
      `is nothing at:\n\n${path}\n\n` +
      `Run \`cargo build\` in the checkout, or set ${OVERRIDE} to the binary ` +
      `to run.`,
  );
}

/// What the human is told when the one address is already somebody else's.
///
/// The Rust app's words, for the same situation and the same reasons
/// (ADR-0012): fronting whatever is there would conflate two Verksteads over
/// possibly different Data Directories, and moving to a free port would leave
/// the bookmark on the phone pointing at the wrong one. Both were rejected
/// twice, so what is left is naming the address and stopping. Not *another copy
/// of this app*, which the single-instance lock has already dealt with by the
/// time these words can be reached.
function foreign(address: string): void {
  dialog.showErrorBox(
    "Verkstead cannot start",
    `Something is already listening on ${address}, which is the one address ` +
      `Verkstead serves on.\n\n` +
      `That will be a Verkstead the machine starts for itself, or one started ` +
      `in a terminal. Stop that one, then start Verkstead again.`,
  );
}

/// The window, once there is one — which a second launch wants and the launch
/// it happens in cannot hand it.
let onscreen: BrowserWindow | undefined;

/// Whether the app is on its way out under its own steam, so that the child
/// going is the expected end of a quit rather than news about the server.
let leaving = false;

async function run(): Promise<void> {
  // The one read of the process's own platform and environment, made first
  // because everything below is a function of these two values — and the
  // sidecar inherits this same environment, so what the server is about to
  // resolve for itself is what these resolve here.
  const machine = { platform: process.platform, env: process.env };

  // And the log file before anything has anything to say, so that every line
  // this run makes is in it — including the one a launch that hands over says
  // on its way out. Where it went it says for itself, on the terminal as well
  // as in the file; a machine with nowhere to put one says that instead, and
  // goes on running.
  keep(logDir(machine));

  // First of the app's own steps, and before anything is started: a second
  // launch of the app is this one's window brought forward, and the launch
  // that asked exits having made nothing. It is also what the probe below
  // rests on — with this held, a listener on the address cannot be another
  // copy of this app.
  if (!app.requestSingleInstanceLock()) {
    say("Verkstead is already running, so this launch hands over to it");
    app.quit();
    return;
  }

  app.on("second-instance", () => {
    say("a second launch — the window already open is brought forward");
    if (onscreen !== undefined) {
      forward(onscreen);
    }
  });

  // Closing the window quits, on every platform including the Mac. Stage 03 is
  // what makes it a choice, with a tray to keep running in and the Dock
  // behaviour that goes with it; until there is one, a window closed with the
  // app still running would be a Verkstead with no way back to itself.
  app.on("window-all-closed", () => app.quit());

  if (await taken(HOST, PORT)) {
    say(`something is already listening on ${ADDRESS}, so there is nothing to start`);
    foreign(ADDRESS);
    app.exit(1);
    return;
  }

  const path = cli({
    packaged: app.isPackaged,
    entry: import.meta.dirname,
    resources: process.resourcesPath,
    ...machine,
  });

  // The directory the server is about to resolve for itself, and so the one the
  // **Workbench Key** is in.
  const data = dataDir(machine);

  // Before anything is started, so that the app which cannot serve has done
  // nothing at all — and before `whenReady`, because there is nothing to wait
  // for in order to put a message on the screen.
  if (!existsSync(path)) {
    missing(path);
    app.exit(1);
    return;
  }

  const sidecar = start(path, heard);
  say(`the sidecar is ${path}, at pid ${sidecar.pid}`);

  // The app quitting is the sidecar stopping. `will-quit` rather than
  // `before-quit` so that a quit which something else has since cancelled does
  // not take the server with it.
  app.on("will-quit", () => {
    leaving = true;
    sidecar.stop();
  });

  // And the server ending is the app quitting, carried over from the tray app:
  // there is nothing for a window to draw once the thing it is a window onto
  // has gone. Cleanly, killed or crashed are all the same end, and which of
  // them it was is said before the app goes — including the crash that a bind
  // the probe above did not catch comes out as.
  void sidecar.gone.then((ending) => {
    if (leaving) {
      return;
    }
    say(`the server has gone — ${how(ending)}, so the app goes with it`);
    app.quit();
  });

  // And a signal is the app quitting, which is what carries the child out with
  // it: an unhandled `SIGTERM` ends this process without running anything, and
  // what it would leave behind is a server holding the one port the next
  // launch needs. Ctrl-C at a terminal reaches the child on its own — the
  // signal goes to the whole process group — but `kill` on the app does not.
  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
    process.once(signal, () => app.quit());
  }

  await app.whenReady();

  try {
    const waited = await healthy(HEALTH);
    say(`the server answered after ${waited} ms`);
  } catch (trouble) {
    if (!(trouble instanceof NeverCameUp)) {
      throw trouble;
    }
    // Said rather than shown: there is no window to draw a dialog over yet, and
    // the line is what a developer running `pnpm start` is reading anyway.
    say(`the server never came up — ${trouble.message}`);
    leaving = true;
    sidecar.stop();
    app.exit(1);
    return;
  }

  if (data === undefined) {
    // Which the server refuses to start over, so health would never have
    // answered and this line is unreachable in practice. Said rather than
    // assumed: it costs one branch, and it buys a window that says why it is
    // sitting on a refusal.
    say("there is nowhere on this machine for a Data Directory, so there is no key to read");
  }

  // The key is read at every load rather than once here: **Reset key** on the
  // phone writes that file while this window is open, and a link built from a
  // secret read at startup is a 401 with extra steps.
  onscreen = open({ origin: ORIGIN, secret: () => (data === undefined ? undefined : keyIn(data)) });
}

// **Started rather than awaited**, and this is not a style. Electron emits
// `ready` only once the entry module has finished evaluating, so a top-level
// `await` here is an app whose `whenReady` never resolves — the sidecar comes
// up, and then nothing else ever happens. Evaluation finishes; the work goes on
// in the promise.
run().catch((trouble: unknown) => {
  say(`the app could not start — ${String(trouble)}`);
  app.exit(1);
});
