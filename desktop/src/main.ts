//! The app itself: the one file that reads the running application.
//!
//! What it does at this stage is the sidecar and nothing around it — find the
//! headless `verkstead`, start it as `serve --desktop`, wait for the server to
//! answer, and own the child's lifetime from there. The window that opens on
//! the workbench is the next task's, and every question this file asks is
//! answered by a module beside it that vitest can run without an Electron —
//! see the wall in `eslint.config.js`.

import { existsSync } from "node:fs";

import { app, dialog } from "electron";

import { cli, OVERRIDE } from "./cli.js";
import { healthy, NeverCameUp } from "./health.js";
import { say } from "./log.js";
import { start } from "./sidecar.js";
import { HEALTH } from "./workbench.js";

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

async function run(): Promise<void> {
  const path = cli({
    packaged: app.isPackaged,
    entry: import.meta.dirname,
    resources: process.resourcesPath,
    platform: process.platform,
    env: process.env,
  });

  // Before anything is started, so that the app which cannot serve has done
  // nothing at all — and before `whenReady`, because there is nothing to wait
  // for in order to put a message on the screen.
  if (!existsSync(path)) {
    missing(path);
    app.exit(1);
    return;
  }

  const sidecar = start(path);
  say(`the sidecar is ${path}, at pid ${sidecar.pid}`);

  // The app quitting is the sidecar stopping. `will-quit` rather than
  // `before-quit` so that a quit which something else has since cancelled does
  // not take the server with it.
  app.on("will-quit", () => sidecar.stop());

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
    // Said rather than shown: the window this would have been drawn over is
    // the next task's, and the line is what a developer running `pnpm start`
    // is reading anyway.
    say(`the server never came up — ${trouble.message}`);
    sidecar.stop();
    app.exit(1);
  }
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
