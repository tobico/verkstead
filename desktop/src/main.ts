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
import { join } from "node:path";

import { app, dialog, type BrowserWindow } from "electron";

import { artwork } from "./artwork.js";
import { FILE } from "./bounds.js";
import { cli, type Install, OVERRIDE } from "./cli.js";
import { closing } from "./closing.js";
import { healthy, NeverCameUp } from "./health.js";
import { keyIn } from "./key.js";
import { heard, keep, say } from "./log.js";
import { shortcuts } from "./menu.js";
import { dataDir, logDir } from "./platform.js";
import { FILE as DESKTOP, settings } from "./settings.js";
import { how, type Sidecar, start } from "./sidecar.js";
import { taken } from "./taken.js";
import { lower, raise } from "./tray.js";
import { forward, open } from "./window.js";
import { ADDRESS, HEALTH, HOST, LISTEN, ORIGIN, PORT } from "./workbench.js";

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

/// Whether a launch asked for the window while there was not one yet.
///
/// A second launch is answered by bringing the window forward, and the window is
/// not there until the server has answered — which is a first start on a cold
/// database, so it is seconds rather than an instant. The launch that asked has
/// already exited by then, so the asking is remembered here and answered by
/// [`open`] instead: pressing the icon twice while Verkstead is coming up is a
/// window that arrives, not a press that went nowhere.
let wanted = false;

/// The sidecar, once there is one — for the ways out that are not a quit.
///
/// `will-quit` is what stops it on every ordinary ending, and that handler is
/// registered over the child itself. This is for the ways out that never reach a
/// quit: `app.exit`, which ends the process without running either the quit
/// events or Node's own `exit` hook, so the failure that takes it has to take the
/// child by hand. See the note at the top of `sidecar.ts`.
let child: Sidecar | undefined;

/// Whether the app is on its way out under its own steam, so that the child
/// going is the expected end of a quit rather than news about the server.
let leaving = false;

/// Whether a quit is already under way, which is what a close arriving during
/// one means.
///
/// **Because a quit closes the window on its way out.** The tray's Quit, Cmd+Q
/// and the sidecar's ending all reach `app.quit`, and every one of them ends up
/// at the same `close` event the close button raises — so an app that read the
/// policy there would hide its window instead of quitting, or put the warning
/// up in front of somebody who had just chosen Quit. The warning is the close
/// button's alone (ADR-0020), and this is what makes that true.
let quitting = false;

/// Stop the sidecar and end this launch with `code`.
///
/// **The stop is the point**, because nothing else here does it. `app.exit` runs
/// neither `will-quit` nor the `exit` hook in `sidecar.ts`, so a refusal that
/// exited without this would leave a child nobody had signalled — and what
/// becomes of one is then down to whether it happens to write again: its stdout
/// is a pipe into this process, so a server still logging takes `SIGPIPE` and a
/// server sitting quietly does not. A sidecar left holding the one address the
/// next launch needs is that launch meeting its own server as a foreign listener
/// and refusing to start, which is far too much to leave to the timing of a log
/// line.
function give(code: number): void {
  leaving = true;
  child?.stop();
  app.exit(code);
}

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
  const kept = keep(logDir(machine));

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
    if (onscreen === undefined) {
      // Which is a launch while this one is still waiting on the server. Kept
      // rather than dropped: the window is what was asked for, and it is a
      // moment away.
      wanted = true;
      say("a second launch, and the window is not open yet — it comes up in front");
      return;
    }

    say("a second launch — the window already open is brought forward");
    forward(onscreen);
  });

  // Every window gone is the app going, and the close policy is what decides
  // whether a close ever reaches this: where closing means keep running, the
  // window is hidden rather than closed and nothing here fires; where it means
  // quit, the window really was the last of the app. A Mac reaches it only on
  // its way out under Cmd+Q, its close always being a hide.
  app.on("window-all-closed", () => app.quit());

  // And a Dock activation is the window coming back, which is the other half of
  // what closing means on a Mac (ADR-0020): the app is a regular Dock app now,
  // so pressing its icon there is the same act as Open on the tray. Registered
  // everywhere, being a Mac's event to emit.
  app.on("activate", () => {
    if (onscreen === undefined) {
      wanted = true;
      return;
    }
    forward(onscreen);
  });

  // `before-quit` rather than `will-quit`: this one comes before the windows
  // are closed, and what it is here for is the close that a quit is about to
  // cause.
  app.on("before-quit", () => {
    quitting = true;
  });

  if (await taken(HOST, PORT)) {
    say(`something is already listening on ${ADDRESS}, so there is nothing to start`);
    foreign(ADDRESS);
    give(1);
    return;
  }

  // Where this app is running from, which is what both of the files it ships
  // beside itself are a function of: the CLI it starts, and the artwork the
  // tray draws.
  const install: Install = {
    packaged: app.isPackaged,
    entry: import.meta.dirname,
    resources: process.resourcesPath,
    ...machine,
  };

  const path = cli(install);

  // The directory the server is about to resolve for itself, and so the one the
  // **Workbench Key** is in.
  const data = dataDir(machine);

  // Before anything is started, so that the app which cannot serve has done
  // nothing at all — and before `whenReady`, because there is nothing to wait
  // for in order to put a message on the screen.
  if (!existsSync(path)) {
    missing(path);
    give(1);
    return;
  }

  // The one setting of the server's the app overrides, said where a developer
  // who exported it will read it: the port is fixed by decision and everything
  // here probes, waits on and loads that one address, so a server sent
  // elsewhere would be a Verkstead nothing in this process could find.
  const elsewhere = machine.env[LISTEN];
  if (elsewhere !== undefined && elsewhere !== "" && elsewhere !== ADDRESS) {
    say(
      `${LISTEN} says ${elsewhere}, and the app serves on ${ADDRESS} — ` +
        `the sidecar is told ${ADDRESS}`,
    );
  }

  const sidecar = start(path, ADDRESS, heard);
  child = sidecar;
  say(`the sidecar is ${path}, at pid ${sidecar.pid}`);

  // The app quitting is the sidecar stopping, and the icon leaving the panel
  // — the two things this app has outside its own process. `will-quit` rather
  // than `before-quit` so that a quit which something else has since cancelled
  // takes neither with it.
  app.on("will-quit", () => {
    leaving = true;
    lower();
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

  // Before the window, so that it is never briefly a window whose keystrokes do
  // nothing: the menu is what registers copy, paste, zoom, reload and the
  // developer tools, and the bar it would be drawn in is hidden by the window.
  shortcuts(machine.platform);

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
    give(1);
    return;
  }

  if (data === undefined) {
    // Which the server refuses to start over, so health would never have
    // answered and this line is unreachable in practice. Said rather than
    // assumed: it costs one branch, and it buys a window that says why it is
    // sitting on a refusal.
    say("there is nowhere on this machine for a Data Directory, so there is no key to read");
  }

  // Electron's own user data, which is this machine's and never the server's:
  // where the window sits and what closing it means are facts about the desk in
  // front of the human, so both are kept beside what Electron keeps here rather
  // than in `config.yaml` (ADR-0020).
  const userData = app.getPath("userData");
  const desk = join(userData, DESKTOP);

  // The key is read at every load rather than once here: **Reset key** on the
  // phone writes that file while this window is open, and a link built from a
  // secret read at startup is a 401 with extra steps.
  const window = open({
    origin: ORIGIN,
    secret: () => (data === undefined ? undefined : keyIn(data)),
    state: join(userData, FILE),

    // Read at the moment of the press, for the reason the key is: the settings
    // file is what the Desktop page writes, and a policy read once at startup
    // would be a radio nobody could see the effect of without a restart. A quit
    // already under way is not a press at all.
    closing: () => (quitting ? "quit" : closing(settings(desk), machine.platform)),
  });
  onscreen = window;

  // And then the icon, which is the other way to this window and the only one
  // while it is off the screen — which is why the close policy falls to Quit
  // without it. Shown unless this machine has said otherwise: turning **Show
  // tray icon** off while the app is running is the bridge's to enact, and what
  // is read here is where the last run left it.
  if (settings(desk).trayIcon) {
    raise({
      icon: artwork(install),
      kept,
      open: () => forward(window),
      quit: () => app.quit(),
    });
  } else {
    say("the desktop settings say no tray icon, so there is none — and closing the window quits");
  }

  // And the launch that asked for the window while there was not one yet, which
  // is a press of the icon over a Verkstead still coming up.
  if (wanted) {
    say("the launch that asked while this one was starting gets the window now");
    forward(window);
  }
}

// **Started rather than awaited**, and this is not a style. Electron emits
// `ready` only once the entry module has finished evaluating, so a top-level
// `await` here is an app whose `whenReady` never resolves — the sidecar comes
// up, and then nothing else ever happens. Evaluation finishes; the work goes on
// in the promise.
//
// **The sidecar goes with it, through [`give`]**: by the time anything here can
// throw there may be a server running, and `app.exit` takes neither the quit
// events nor Node's `exit` hook with it.
run().catch((trouble: unknown) => {
  say(`the app could not start — ${String(trouble)}`);
  give(1);
});
