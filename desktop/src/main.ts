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
//! **And it is what answers the page**, over the channels
//! [`bridge.ts`](./bridge.js) names: the settings read, a set enacted in the
//! run it arrives in, the log file opened, the startup registration read and
//! written, whether this desktop shows notifications at all, and the window dressed to whatever head the page turned out to have
//! drawn — its controls overlay recoloured, or a Mac's traffic lights moved into
//! the row that head stands in. Each of them is something only this process can do — a file
//! under Electron's user data, an icon on somebody's panel, a file handed to
//! whatever the desktop reads text with, a login item registered, a window
//! redressed — while what a set *means* is [`changed`](./settings.js)'s, what a
//! registration *is* is [`startup.ts`](./startup.js)'s and what a head *is* is
//! [`decorations.ts`](./decorations.js)'s, all of which vitest runs.
//!
//! **The order at the top of [`run`] is the lifecycle**, and it is an order
//! rather than a sequence of conveniences: the log file, so that every line
//! below it is in the file somebody will be asked to send; then the lock, so
//! that a second launch is the first window brought forward and never a second
//! sidecar; then the **Install Offer**, so that a download hands over to its
//! installed copy having started nothing; then the address, which the lock is
//! what makes an unambiguous question; then the binary; and only then a child. Everything before the
//! child is an app that can refuse having made nothing at all.
//!
//! **And it holds the notices stream for the rest of the run**, drawing each
//! notice the phones get as the platform's own notification while the window
//! is not in front — Web Push being refused inside Electron, which ships
//! without Google's push client. Reading the stream and the showing rule are
//! [`notices.ts`](./notices.js)'s, which vitest runs; the `Notification` is
//! here.
//!
//! **And a launch may be a login's rather than a human's.** The flag
//! [`HIDDEN`](./startup.js) is what says so where the registration is a command
//! line, and the platform itself is what says so on a Mac, whose login item
//! carries no arguments; either way what it comes to is a window that stays off
//! the screen while there is an icon to reach the app by —
//! [`hidden`](./startup.js) is that whole reading, and the registration it was
//! read out of is rewritten here at every launch while there is one.

import { spawn } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";

import {
  app,
  BrowserWindow,
  dialog,
  ipcMain,
  type MessageBoxOptions,
  type MessageBoxReturnValue,
  Notification,
} from "electron";
import electronUpdater from "electron-updater";

import { artwork } from "./artwork.js";
import { AUTOSCROLLING } from "./autoscroll.js";
import { FILE } from "./bounds.js";
import {
  ASKED,
  HEAD,
  INSTALLS,
  LOGS,
  NOTIFIABLE,
  PLACED,
  PRELOAD,
  type Pressed,
  REGISTER,
  SET,
  STARTUP,
  WHERE,
} from "./bridge.js";
import { cli, type Install, OVERRIDE } from "./cli.js";
import { closing } from "./closing.js";
import { buttoned, lights, overlaid, overlay, worn } from "./decorations.js";
import { healthy, NeverCameUp } from "./health.js";
import { keyIn } from "./key.js";
import { takeOver as takeOverAgent } from "./launchd.js";
import { heard, keep, type Kept, say } from "./log.js";
import { grey, shortcuts } from "./menu.js";
import { hold, type Notice, opens, showing } from "./notices.js";
import {
  asking,
  executable,
  failure,
  HICOLOR,
  install as installTo,
  INSTALL,
  installed,
  INSTALLING,
  instead,
  moveInto,
  newer,
  NotInstalled,
  offered,
  OPENING,
  type Placed,
  placed,
  placement,
  plan,
  relaunched,
  version,
} from "./offer.js";
import { dataDir, logDir } from "./platform.js";
import { GUIDE, ISSUES } from "./roles.js";
import { takeOver as takeOverValue } from "./runkey.js";
import { changed, FILE as DESKTOP, set, type Settings, settings } from "./settings.js";
import { how, type Sidecar, start } from "./sidecar.js";
import {
  APP_ID,
  hidden,
  type LoginItem,
  type Registering,
  type Registration,
  startup,
  type Startup,
} from "./startup.js";
import { taken } from "./taken.js";
import { opening } from "./opening.js";
import { logs, lower, raise, type Trayed } from "./tray.js";
import { APPDIR, type Environment, stripped, unmounted } from "./unmounted.js";
import {
  alive,
  ASKING,
  HANDED_OVER,
  missed,
  prepare,
  RELAUNCH,
  remember,
  told,
  update,
  waitOut,
} from "./updating.js";
import { browse, forward, open, type Opened } from "./window.js";
import { enabled, NOWHERE, type Pane, pathFor, type Where, where } from "./where.js";
import {
  ADDRESS,
  HEALTH,
  HOST,
  LISTEN,
  NOTICES,
  ORIGIN,
  PORT,
  UPDATE_FAILED,
  UPDATE_PROGRESS,
} from "./workbench.js";

/// What the human is told when the CLI is not where the app looked.
///
/// The path is the whole of what they need: a developer who has not run cargo
/// yet is the common case, and one running against a binary somewhere else is
/// the variable. Said in a dialog rather than on stdout because an app started
/// from an icon has no terminal for a line to land in.
function missing(path: string): void {
  dialog.showErrorBox(
    "Verkstead isn't built.",
    `There's no verkstead binary at:\n\n${path}\n\n` +
      `Run cargo build in the checkout, or set ${OVERRIDE} to the binary to run.`,
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
    "Verkstead can't start.",
    `Something is already listening on ${address}, the one address Verkstead ` +
      `serves on.\n\n` +
      `That's likely a Verkstead the machine starts for itself, or one started ` +
      `in a terminal. Stop it, then start Verkstead again.`,
  );
}

/// What the human is told when the server was started and never answered.
///
/// Every such start, not only the one after an Update: the app was opened
/// from an icon more often than not, and a launch that exits with nothing on
/// the screen is one somebody presses again and again. The log is named
/// because what the server said on its way down is in it.
function silent(kept: Kept, waited: number): void {
  const seconds = Math.round(waited / 1_000);
  const where =
    "file" in kept
      ? `What it said is in the log at:\n\n${kept.file}`
      : "What it said went to the standard error of whatever started Verkstead.";

  dialog.showErrorBox(
    "Verkstead's server didn't start.",
    `It didn't answer within ${seconds} seconds of being started.\n\n${where}`,
  );
}

/// The window, once there is one — which a second launch wants and the launch
/// it happens in cannot hand it.
let onscreen: BrowserWindow | undefined;

/// How to go somewhere on the workbench in that window, once there is one — what
/// the menu bar's Settings… and New Conversation reach at the press, the menu
/// being set before the window is opened.
let visiting: Opened["visit"] | undefined;

/// What the open Conversation can go to, as the page last said it — what the
/// Mac's Go menu is greyed and enabled by, and nowhere until the page has said
/// anything. See [`where`](./where.js).
let going: Where = NOWHERE;

/// Whether a launch asked for the window while there was not one yet.
///
/// A second launch is answered by bringing the window forward, and the window is
/// not there until the server has answered — which is a first start on a cold
/// database, so it is seconds rather than an instant. The launch that asked has
/// already exited by then, so the asking is remembered here and answered by
/// [`open`] instead: pressing the icon twice while Verkstead is coming up is a
/// window that arrives, not a press that went nowhere.
///
/// **`second-instance`'s alone.** A Dock activation is the other thing that
/// arrives before there is a window, and it is not an ask: Electron emits
/// `activate` at a first launch as well as at a press, so a remembered one is a
/// hidden login start showing its window anyway. That handler is registered once
/// there is a window to bring forward, and never sets this — see the note beside
/// it.
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

/// Whether this quit is an Install handing over with the sidecar running —
/// pressed on the Desktop page — so that the quit waits for the sidecar to have
/// gone before the process does.
///
/// **Because the copy starts the moment this process has gone**: Electron's
/// relaunch and a Mac's move both wait for this pid and nothing else, and a
/// sidecar still stopping is the copy meeting the address taken and refusing to
/// start. At launch there is no sidecar yet, and an exit is all there is.
let relaunching = false;

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

/// Enact a set that arrived over the bridge, and answer with the settings in
/// force once it has been.
///
/// **In this run rather than at the next launch**, which is what makes the
/// Desktop page a control rather than a form: the icon goes or comes back as
/// the switch is moved, and what a close means changes with it — the policy is
/// read out of the file at the moment of the press, and the notifications
/// switch at the moment of each notice, so there is nothing to re-read here.
///
/// **What is refused writes nothing.** [`changed`](./settings.js) is what says
/// so, and what comes back then is the settings as they stand — which is a
/// control on the page that goes back where it was rather than one left showing
/// something the app never agreed to.
///
/// **And the answer is read back off the file rather than handed back.** A
/// write that failed is a set that did not happen, and the app goes on behaving
/// the way it was behaving; the page is told what is true, and the icon follows
/// the same reading.
function enact(desk: string, trayed: Trayed, sent: unknown): Settings {
  const wanted = changed(sent);

  if (wanted === undefined) {
    say("a set came over the bridge that is not one of this app's settings, so nothing is written");
    return settings(desk);
  }

  // Said before it is done, so that a write which then failed reads as what it
  // is: this line, and the settings module's own saying it could not keep them.
  say(
    `the Desktop page set ${Object.entries(wanted)
      .map(([which, value]) => `${which} to ${String(value)}`)
      .join(" and ")}`,
  );

  set(desk, { ...settings(desk), ...wanted });

  const now = settings(desk);

  // Where the icon should now be, said as a statement rather than as a change:
  // both of these leave an icon that is already as asked exactly where it is.
  if (now.trayIcon) {
    raise(trayed);
  } else {
    lower();
  }

  return now;
}

/// Enact a tick of **Launch on Startup** that arrived over the bridge, and
/// answer with how it stands once it has been.
///
/// **The registration is the state** (Set 846 Q9a), so there is nothing to write
/// anywhere else and nothing to keep in step: what comes back is read off the
/// platform again, which is a box that says what is true rather than what was
/// pressed. A platform that refused the registration says so in the answer — see
/// [`Registration`](./startup.js) — rather than throwing at a renderer.
///
/// **And what is not a yes or a no is nothing at all.** The same reading
/// [`enact`] makes of a set: a bridge is not to be trusted with the shape just
/// because the window is the app's own.
function ticked(starts: Startup, asked: unknown): Registration {
  if (typeof asked !== "boolean") {
    say(
      "a tick came over the bridge for Launch on Startup that is neither yes nor no, " +
        "so nothing is registered",
    );
    return starts.standing();
  }

  say(`the Desktop page asked for Launch on Startup ${asked ? "on" : "off"}`);

  return starts.set(asked);
}

/// Wear what the page says its head is: the window's controls overlay recoloured
/// and resized to the band beneath it, or a Mac's traffic lights moved into the
/// row that head stands in, while the window is open.
///
/// **The one thing on this bridge the page says rather than asks** (Set 847
/// Q11b). The strip the platform draws its controls on belongs to a window with
/// no title bar, so it is the app's to paint — and what to paint it is the page's
/// to know: the viewer has a light scheme and a dark one, the head is drawn on
/// the paper of whichever the machine is in, and the band is written in rem
/// against a root font size only that browser can be asked about. So this
/// answers the push by doing as it is told, and has no opinion about any of the
/// three values.
///
/// **And on a Mac it moves the traffic lights instead** (Set 889 Q4a). There is no
/// overlay there — `setTitleBarOverlay` throws *"Titlebar overlay is not
/// enabled"*, see [`overlaid`](./decorations.js) — and what that platform has in
/// its place is three buttons the app can be told where to put. So the same push
/// is answered by `setWindowButtonPosition`, at the point
/// [`lights`](./decorations.js) works out of the row the page said it drew: the
/// lights follow a larger text size exactly as the overlay's height does, neither
/// of them a constant compiled into the app.
///
/// **And a push that is not a head changes nothing**, the same reading [`enact`]
/// and [`ticked`] make of what arrives: what an unchecked one reaches is a call
/// that throws at a colour it cannot parse.
function wearing(window: BrowserWindow | null, platform: NodeJS.Platform, pushed: unknown): void {
  const head = worn(pushed);

  if (head === undefined) {
    say("a head came over the bridge that is not one, so the window is left as it was");
    return;
  }

  if (window === null) {
    // A push from a window that has since gone. Nothing to move and nothing to
    // recolour, and nothing the matter either — the page said it on its way out.
    return;
  }

  if (buttoned(platform)) {
    const point = lights(head);

    say(
      `the page's head has its row ${head.middle}px down, ` +
        `so the traffic lights sit at ${point.x},${point.y}`,
    );
    window.setWindowButtonPosition(point);
    return;
  }

  if (!overlaid(platform)) {
    // A platform with neither, which is none of the three this runs on.
    return;
  }

  say(`the page's head is ${head.paper} with ${head.ink} marks, ${head.band}px tall`);
  window.setTitleBarOverlay(overlay(head));
}

/// The notifications on the screen, held until each is clicked or closed.
///
/// **Held because a notification nobody references is collected**, and its
/// click handler with it: Electron's `Notification` is a JavaScript object over
/// the platform's, and one that has gone out of scope by the time it is clicked
/// is a click that does nothing.
const onDesk = new Set<Notification>();

/// Draw a notice as the platform's own notification, unless the window is in
/// front — which already draws the news — and when it is clicked, open what it
/// is about and bring the window forward, the three steps the tray's Open takes.
///
/// What it is about is where a tap on the phone lands: the page is told the path
/// and routes there in place, or the window is loaded on it where there is no
/// page to tell yet — see [`Opened.visit`](./window.js).
///
/// The sentence is the title and the repo the body, with the platform's own
/// sound. Every notice is logged whichever way it went, so that a notification
/// somebody did not see is a line saying why.
function noticed({ window, visit }: Opened, arrived: Notice, on: boolean): void {
  const verdict = showing(
    {
      visible: window.isVisible(),
      minimized: window.isMinimized(),
      focused: window.isFocused(),
    },
    on,
  );

  if (!verdict.show) {
    say(`a notice is withheld, because ${verdict.why} — ${arrived.title}`);
    return;
  }

  if (!Notification.isSupported()) {
    say(`a notice cannot be shown, because this desktop has no notifications — ${arrived.title}`);
    return;
  }

  const shown = new Notification({
    title: arrived.title,
    body: arrived.project ?? "",
    silent: false,
  });

  shown.on("click", () => {
    onDesk.delete(shown);
    say(`a notification was clicked, so the window opens what it is about — ${arrived.title}`);
    visit(opens(arrived));
    forward(window);
  });
  shown.on("close", () => onDesk.delete(shown));
  shown.on("failed", (_event, error) => {
    onDesk.delete(shown);
    say(`a notification failed — ${error}`);
  });

  onDesk.add(shown);
  shown.show();
  say(`a notice is shown — ${arrived.title}`);
}

/// Install the app at `from` and relaunch from the copy — the **Install
/// Offer**'s **Install**, and `--install`'s — answering whether this launch is
/// over.
///
/// **On a Mac the whole of it is Electron's** `moveToApplicationsFolder`, which
/// moves the bundle into `/Applications` and quits and relaunches on its own;
/// what is here is its answer read, by [`moveInto`], and the failure dialog.
/// Its conflict handler is left to its default, which trashes a copy that was
/// there.
///
/// **On Linux what the move and the writes are is [`offer.ts`](./offer.js)'s**; what is
/// here is the reads of the running image it needs, the failure dialog, and the
/// relaunch. Nothing has been started yet — the sidecar comes after this — so
/// handing over is an exit and nothing more, and Electron's relaunch waits for
/// this process to have gone before the copy starts, which is what lets the
/// copy take the lock.
///
/// **A newer Verkstead already installed is offered to open instead** — see
/// [`superseded`] — and an older or the same one is replaced.
///
/// **A failure carries on from where it is** (ADR-0020): the dialog says why,
/// and this launch goes on serving out of the download.
async function installing(
  from: string,
  machine: { platform: NodeJS.Platform; env: Environment },
  asked: boolean,
  window?: BrowserWindow | null,
): Promise<boolean | NotInstalled> {
  const over = await superseded(machine, asked, window);
  if (over !== undefined) {
    return over;
  }

  if (machine.platform === "darwin") {
    // After ready, as the rest of Electron's AppKit is: the move may put up
    // the administrator's password dialog.
    await app.whenReady();
    say(`installing ${from} into /Applications`);

    // Before the move rather than after it, because the quit the move starts is
    // Electron's own and may already be under way when the call returns.
    relaunching = child !== undefined;

    try {
      // A copy already there is replaced: `superseded` has read its version
      // and found it older or the same, so the conflict is no question.
      moveInto(() => app.moveToApplicationsFolder({ conflictHandler: () => true }));
    } catch (trouble) {
      relaunching = false;
      return failed(trouble, from, machine);
    }

    // Electron has moved the bundle and relaunches the copy once this process
    // has gone — so it goes: at once with nothing started, and once the
    // sidecar has stopped where there is one.
    say("installed, so this launch hands over to the copy in /Applications");
    if (child === undefined) {
      app.exit(0);
    } else {
      departing();
    }
    return true;
  }

  // The icon set, read out of the mount this run came from: the only place the
  // icons are, and gone once the app runs from anywhere else.
  const appdir = machine.env[APPDIR];
  let found: string[] = [];
  if (appdir !== undefined) {
    try {
      found = readdirSync(join(appdir, HICOLOR), { recursive: true, encoding: "utf8" });
    } catch (trouble) {
      say(`the image's icons could not be listed, so none are installed — ${String(trouble)}`);
    }
  }

  const planned = plan(from, machine, appdir, found);
  if (planned === undefined) {
    say("there is nowhere on this machine to install to, so this launch carries on");
    return false;
  }

  say(`installing ${from} as ${planned.file}, with ${planned.entry} and ${planned.icons.length} icons`);

  try {
    installTo(planned, machine);
  } catch (trouble) {
    return failed(trouble, from, machine);
  }

  say(`installed, so this launch hands over to ${planned.file}`);
  handOver(planned.file, machine);
  return true;
}

/// Whether a newer Verkstead is already installed, and what came of it: true
/// is this launch handed over to it, false is **Keep this one** — the launch
/// carrying on from where it is, with nothing installed — and nothing is an
/// Install to go ahead with, over an older copy, the same one, one whose
/// version cannot be read, or none.
///
/// Asked, the open-installed dialog says so; `--install`, with nobody to ask,
/// opens the newer copy and says so in the log.
async function superseded(
  machine: { platform: NodeJS.Platform; env: Environment },
  asked: boolean,
  window?: BrowserWindow | null,
): Promise<boolean | undefined> {
  const there = installed(machine);
  if (there === undefined || !existsSync(there)) {
    return undefined;
  }

  const was = version(there, machine);
  const running = app.getVersion();
  if (was === undefined || !newer(was, running)) {
    say(`${there} is ${was ?? "of a version that cannot be read"}, not newer than ${running}, so it is replaced`);
    return undefined;
  }

  if (asked) {
    say(`${there} is ${was}, newer than ${running}, so the launch offers to open it instead`);
    const answer = await answered(instead(was, running), window);
    if (answer.response !== OPENING) {
      say("Keep this one is pressed, so this launch carries on from where it is");
      return false;
    }
  } else {
    say(`${INSTALL} finds ${there} at ${was}, newer than ${running}, so it opens that instead`);
  }

  say(`this launch hands over to the installed ${there}`);
  handOver(executable(there, machine.platform), machine);
  return true;
}

/// End this launch, and start `exe` in its place.
///
/// The copy is relaunched with the environment this run was given rather than
/// the one its mount made of it — the same reading the sidecar is handed, see
/// [`unmounted`]. The copy's own runtime puts its own mount in front again.
/// At launch nothing has been started yet, so the exit is all there is to stop.
/// Pressed on the Desktop page, the sidecar is running, and the quit is
/// [`departing`]'s.
///
/// **A run out of an AppImage starts the copy itself**, detached, as
/// electron-updater starts an Update's image, rather than through Electron's
/// relaunch. That relaunch is made by a helper process running this run's own
/// Electron out of the mount, after this process has gone — and with it the
/// runtime's FUSE mount, so on a runner with FUSE the copy never started and
/// logged nothing (Release run 41245808128). `--appimage-extract-and-run`,
/// whose extracted tree outlives the exit, hid it. Started this way the copy
/// comes up while this run still holds the lock, so it is told this pid and
/// waits it out — see [`waitOut`]. Elsewhere Electron's relaunch waits for
/// this process to have gone, which is what lets the copy take the lock.
function handOver(exe: string, machine: { platform: NodeJS.Platform; env: Environment }): void {
  const mounted = machine.env[APPDIR] !== undefined;
  const passed = unmounted(machine.env);
  for (const variable of stripped(machine.env, passed)) {
    if (passed[variable] === undefined) {
      delete process.env[variable];
    } else {
      process.env[variable] = passed[variable];
    }
  }

  // Told before the relaunch is asked for, which takes this environment as it
  // stands — see [`departing`].
  if (child !== undefined || mounted) {
    process.env[RELAUNCH] = String(process.pid);
  }

  if (mounted) {
    const copy = spawn(exe, relaunched(process.argv), {
      env: process.env,
      stdio: "ignore",
      detached: true,
    });
    copy.on("error", (trouble: unknown) => {
      say(`${exe} could not be started — ${String(trouble)}`);
    });
    copy.unref();
  } else {
    app.relaunch({ execPath: exe, args: relaunched(process.argv) });
  }

  if (child === undefined) {
    app.exit(0);
  } else {
    departing();
  }
}

/// Quit for an Install with the sidecar running, which goes with the quit: the
/// sessions on it stop, and the copy's own server resumes them.
///
/// The quit waits for the sidecar to have gone — see [`relaunching`]. On Linux
/// the copy is told this pid as well, as an Update's relaunch is, so that it
/// waits out the address too — see [`waitOut`]; [`handOver`] says it before
/// asking for the relaunch, which takes the environment as it stands at the
/// call. A Mac's move relaunches through `open`, which hands the copy none of
/// this environment, so there the wait is the quit's alone.
function departing(): void {
  relaunching = true;
  leaving = true;
  quitting = true;
  app.quit();
}

/// A native message box, and its answer: before any window at launch, and over
/// `window` where the Desktop page asked for it.
///
/// **A timer ticks while the box is up, and it is load-bearing.** Measured
/// on the packed AppImage under Electron 43.7.5 on X11: the box closed at
/// the press, and the promise it answers with resolved only when something
/// else next woke the main process — some 30 seconds after launch, or never
/// — because this early in a launch nothing else is running to. A tick
/// every tenth of a second is that something, so the press is acted on at
/// once. `showMessageBoxSync` would need no timer and cannot say whether the
/// box was ticked.
async function answered(
  options: MessageBoxOptions,
  window?: BrowserWindow | null,
): Promise<MessageBoxReturnValue> {
  // The message box wants a ready app; `whenReady` answers at once after.
  await app.whenReady();
  const awake = setInterval(() => undefined, 100);
  const shown = window ? dialog.showMessageBox(window, options) : dialog.showMessageBox(options);
  return shown.finally(() => clearInterval(awake));
}

/// The offer put up, and what came of it: true is this launch handed over,
/// false is nothing installed — a Skip or a Keep this one — and a
/// [`NotInstalled`] is an install that failed, which the failure dialog has
/// said. The launch's offer and the Desktop page's Install both come here, the
/// page's with the window it was pressed in and its sentence about the
/// sessions running.
async function offering(
  from: string,
  machine: { platform: NodeJS.Platform; env: Environment },
  desk: string,
  window?: BrowserWindow | null,
  stopping?: string,
): Promise<boolean | NotInstalled> {
  const answer = await answered(asking(from, machine, stopping), window);

  if (answer.checkboxChecked) {
    say("Don't ask again is ticked, so the offer is not made again");
    set(desk, { ...settings(desk), installOffer: false });
  }

  if (answer.response !== INSTALLING) {
    say("the offer is skipped, and this launch carries on from where it is");
    return false;
  }

  return installing(from, machine, true, window);
}

/// **Install** pressed on the Desktop page (ADR-0020, *Amended: a download
/// offers to install itself*): the launch's own dialog, over the window it was
/// pressed in, and what came of it for the row to say — an Install that went
/// ahead answers nothing anybody reads, the app being on its way out.
///
/// The sentence arriving from the page is said only where it is one: anything
/// else is the dialog the launch puts up.
async function pressedInstall(
  here: Placed,
  machine: { platform: NodeJS.Platform; env: Environment },
  desk: string,
  window: BrowserWindow | null,
  stopping: unknown,
): Promise<Pressed> {
  if (!("outside" in here)) {
    say("Install is pressed on the desktop page, and there is nothing to install");
    return {};
  }

  say(`Install is pressed on the desktop page, so ${here.outside} offers to install itself`);
  const came = await offering(
    here.outside,
    machine,
    desk,
    window,
    typeof stopping === "string" ? stopping : undefined,
  );

  return came instanceof NotInstalled ? { failed: came.message } : {};
}

/// An install that did not happen: said in the log and in the failure dialog,
/// and this launch carries on from where it is (ADR-0020).
async function failed(
  trouble: unknown,
  from: string,
  machine: { platform: NodeJS.Platform; env: Environment },
): Promise<NotInstalled> {
  if (!(trouble instanceof NotInstalled)) {
    throw trouble;
  }
  say(`the install did not happen, so this launch carries on from ${from} — ${trouble.message}`);
  await app.whenReady();
  const told = failure(trouble.message, from, machine);
  dialog.showErrorBox(told.title, told.content);
  return trouble;
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

  // And which release this is, and which process: the line an Update is read
  // back by, before and after its relaunch.
  say(`Verkstead ${app.getVersion()} is starting, at pid ${process.pid}`);

  // And the name Windows knows this application by, said before there is a
  // window to be named — which is what ties the window to the Start-menu entry
  // the installer wrote (ADR-0020). That entry carries `System.AppUserModel.ID =
  // net.tobico.Verkstead`, and a window whose own id is something else is a
  // second, unnamed button on the taskbar beside it. Measured on Windows 11
  // under Electron 43 against the packed app: with nothing said here the window
  // carried no id at all and the process answered to `electron.app.Verkstead`,
  // so only a launch made *through* the shortcut grouped — the installer's own
  // run-after-finish, the Run key at login and the launcher double-clicked in
  // the install directory each drew a button of their own.
  //
  // [`APP_ID`](./startup.js)'s, which is the same string the registrations are
  // named for and the same one `electron-builder.yml` packs as the `appId`:
  // one name for the entry, the login item and the window. It is the Windows
  // arm of what `syncDesktopName` does on Linux, and Electron implements the
  // call on Windows alone.
  if (machine.platform === "win32") {
    app.setAppUserModelId(APP_ID);
    say(`this window is ${APP_ID} on the taskbar, which is the Start-menu entry's own name`);
  }

  // And the one thing Chromium has to be told before a renderer of it starts:
  // **Middle-Click Autoscroll**, which Chromium has and enables on Windows
  // alone — so the same workbench scrolled under a held middle button there and
  // did nothing anywhere else. A Blink runtime feature is read by the renderer
  // as it starts rather than asked for per window, so this is the command line
  // rather than the per-window `webPreferences.enableBlinkFeatures` — said once
  // here, before there is a renderer to read it, for every window this app will
  // ever open. Said on every platform and with no branch above it, for the
  // reason [`autoscroll.ts`](./autoscroll.js) gives: a gesture is the same
  // gesture wherever there is a button to hold, and Windows is being told what
  // is already true there.
  app.commandLine.appendSwitch(AUTOSCROLLING.name, AUTOSCROLLING.value);
  say(`a held middle button autoscrolls the window — --${AUTOSCROLLING.name}=${AUTOSCROLLING.value}`);

  // First of the app's own steps, and before anything is started: a second
  // launch of the app is this one's window brought forward, and the launch
  // that asked exits having made nothing. It is also what the probe below
  // rests on — with this held, a listener on the address cannot be another
  // copy of this app.
  // **A relaunch waits out the app it replaces** before it asks for the lock
  // or the address, both of which that app still holds while it quits — see
  // [`waitOut`]. Nothing to wait for on any other launch. The variable is
  // this launch's to read and nobody's after it, the sidecar included.
  const replaced = await waitOut(machine.env, { alive, taken: () => taken(HOST, PORT) });
  if (replaced !== undefined) {
    say(`this launch replaces an app that updated itself, and waited ${replaced} ms for it to go`);
  }
  delete machine.env[RELAUNCH];

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

  // `before-quit` rather than `will-quit`: this one comes before the windows
  // are closed, and what it is here for is the close that a quit is about to
  // cause.
  app.on("before-quit", () => {
    quitting = true;
  });

  // Electron's own user data, which is this machine's and never the server's:
  // where the window sits and what closing it means are facts about the desk in
  // front of the human, so both are kept beside what Electron keeps here rather
  // than in `config.yaml` (ADR-0020). Read here, ahead of the offer below, whose
  // **Don't ask again** is kept in the same file.
  const userData = app.getPath("userData");
  const desk = join(userData, DESKTOP);

  // **Launch on Startup**, which is the one setting on the Desktop page that is
  // not in that file at all: the platform's own registration is the state (Set
  // 846 Q9a), and what is read off this process for it is whether this is a
  // packed app, where its executable is and what `$APPIMAGE` says. The
  // login-item API is handed in rather than reached for, so that the two arms
  // this machine will never take are still arms vitest runs. Made here because
  // whether this launch is a hidden one is what the offer asks it — making it
  // reads nothing, and every reading is made where it is asked.
  const registering: Registering = {
    packaged: app.isPackaged,
    exe: process.execPath,
    ...machine,
  };

  // Four reads of the one call and one write of the other, each of them
  // mechanical: which of them a platform is asked, and what its answer comes to,
  // is `startup.ts`'s — including the one this file cannot make look the same on
  // both, `openAtLogin` being Windows' answer about a value named after the
  // AppUserModelId rather than about the one this app writes.
  const login: LoginItem = {
    registered: () => app.getLoginItemSettings().openAtLogin,

    // Which is why Windows is handed its own list instead: every Run value that
    // would start this executable at a sign-in, Verkstead's own among them.
    // Undefined on a Mac, which carries no such list.
    values: () => app.getLoginItemSettings().launchItems ?? [],

    // The Mac's own account of this launch, which is what it has instead of the
    // flag the other two carry on their command lines.
    openedAtLogin: () => app.getLoginItemSettings().wasOpenedAtLogin,

    // And what macOS 13's `SMAppService` says about the registration beside
    // `openAtLogin`: a human who switched Verkstead off in System Settings is
    // `requires-approval`, which is a registration that starts nothing.
    status: () => app.getLoginItemSettings().status,

    register: (asked) => app.setLoginItemSettings(asked),
  };

  const starts = startup(registering, login);

  // **The Install Offer** (ADR-0020, *Amended: a download offers to install
  // itself*): after the lock, so that an installed copy already running has
  // been handed the launch before any dialog could be drawn, and before the
  // port probe and the sidecar, so that nothing has been started when this
  // launch hands over to the copy. `--install` is its Install with nobody
  // asked; everything else is [`offered`]'s to decide, and every no is said.
  const here = placed({
    packaged: app.isPackaged,
    exe: process.execPath,
    applied: machine.platform === "darwin" && app.isInApplicationsFolder(),
    ...machine,
  });

  if (process.argv.includes(INSTALL)) {
    if ("outside" in here) {
      say(`${INSTALL} asks for an install with nobody asked`);
      if ((await installing(here.outside, machine, false)) === true) {
        return;
      }
    } else {
      say(
        `${INSTALL} asks for an install, and there is nothing to do — ` +
          ("inside" in here ? `it already runs from ${here.inside}` : here.nowhere),
      );
    }
  } else {
    const offer = offered(
      here,
      () => hidden(process.argv, settings(desk), starts.atLogin()),
      settings(desk),
    );

    if ("why" in offer) {
      say(`no Install Offer — ${offer.why}`);
    } else {
      // Nothing below this needs the app not to be ready: `whenReady` is
      // awaited again further down and answers at once.
      say(`this run is ${offer.offer}, outside Applications, so it offers to install itself`);
      if ((await offering(offer.offer, machine, desk)) === true) {
        return;
      }
    }
  }

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

  // And the environment the sidecar gets, which is this process's own with the
  // AppImage's doing taken out of it — see [`unmounted`]. Said where it did
  // something, because a bundle's directories reaching a session's `PATH` or a
  // host binary's loader is the kind of trouble nobody thinks to suspect the app
  // of.
  const passed = unmounted(machine.env);
  const stripping = stripped(machine.env, passed);
  if (stripping.length > 0) {
    say(
      `this run came out of ${machine.env[APPDIR]}, so the sidecar is handed ` +
        `${stripping.join(", ")} with the mount taken out of them`,
    );
  }

  // And how this machine opens a file or a link, which on Linux is the app's own
  // doing rather than `shell`'s — see [`opening`](./opening.js), which says why,
  // and which hands what it starts the environment above. Read here, beside the
  // sidecar's, because it is the same answer to the same question: everything
  // this app starts gets the mount taken out of what it was given.
  const by = opening(machine);
  if (by !== undefined) {
    say(`what opens a file or a link on this machine is ${by.program}`);
  }

  // **This device's Update is asked for on the sidecar's stdout** (ADR-0027),
  // which is read here a line at a time already: the line is filed like any
  // other, and the one the server says for a press of Update is acted on too
  // — see [`update`]. Once per run: the server marks itself updating at the
  // press and says the line once, and an app that has handed over is going.
  let pressed = false;
  const said = (line: string): void => {
    heard(line);
    if (line !== ASKING || pressed) {
      return;
    }
    pressed = true;

    const { autoUpdater } = electronUpdater;
    prepare(autoUpdater, machine.env, say);
    const secret = () => (data === undefined ? undefined : keyIn(data));
    void update({
      updater: autoUpdater,
      report: (percent) => told(UPDATE_PROGRESS, secret(), { percent }),
      fail: (why) => told(UPDATE_FAILED, secret(), { why }),
      // The sidecar going from here is the quit `quitAndInstall` starts rather
      // than news, the launch it starts is to wait this one out, and the
      // version is what that launch checks it came up on — see [`missed`].
      //
      // **And the window's close is a quit's from here**, on every platform:
      // on a Mac `quitAndInstall` is Squirrel's, which closes every window
      // before `before-quit` fires, and a Mac's close is always a hide — so a
      // close left to the policy would cancel the quit, and with it the
      // install.
      handOver: (version) => {
        leaving = true;
        quitting = true;
        process.env[RELAUNCH] = String(process.pid);
        remember(join(app.getPath("userData"), HANDED_OVER), version, say);
      },
      say,
    }).then((outcome) => {
      if (outcome.kind !== "handed-over") {
        pressed = false;
      }
    });
  };

  const sidecar = start(path, ADDRESS, said, passed);
  child = sidecar;
  say(`the sidecar is ${path}, at pid ${sidecar.pid}`);

  // The app quitting is the sidecar stopping, and the icon leaving the panel
  // — the two things this app has outside its own process. `will-quit` rather
  // than `before-quit` so that a quit which something else has since cancelled
  // takes neither with it.
  let waiting = false;
  app.on("will-quit", (event) => {
    leaving = true;
    // Said before either is done, so that the file somebody sends ends with the
    // app going rather than simply stopping: Cmd+Q on a Mac takes the whole app
    // with no dialog and no close of anybody's making, and a log whose last line
    // is whatever happened to be logged first says nothing about which of the
    // ways out this was.
    say("Verkstead is quitting — the icon comes down and the sidecar is stopped");
    lower();
    sidecar.stop();

    // And an Install handing over goes once the sidecar has — see
    // [`relaunching`]. Bounded by the sidecar's own grace, after which it is
    // taken.
    if (relaunching) {
      event.preventDefault();
      if (!waiting) {
        waiting = true;
        void sidecar.gone.then(() => app.exit(0));
      }
    }
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
  //
  // The acts the Mac's bar carries go somewhere in the workbench, and they go
  // there the way a clicked notification does: told to the page, which routes
  // in place, and the window brought forward — a ⌘W'd window is in the Dock.
  const go = (path: string): void => {
    if (visiting === undefined || onscreen === undefined) {
      say(`the menu asked for ${path} before there was a window to open it in`);
      return;
    }
    visiting(path);
    forward(onscreen);
  };
  //
  // The Go menu's panes go where the page last said they are, read at the
  // press; one the page has not named is an item greyed a moment too late.
  const pane = (which: Pane) => (): void => {
    const path = pathFor(going, which);
    if (path === undefined) {
      say(`the menu asked for ${which}, which the open conversation has not got`);
      return;
    }
    go(path);
  };

  // And Back and Forward are the window's own history, which on a page that
  // routes in place is every pane and page walked through.
  const history = () => onscreen?.webContents.navigationHistory;

  const bar = shortcuts(machine.platform, {
    settings: () => go("/settings"),
    compose: () => go("/compose"),
    guide: () => browse(GUIDE, by),
    logs: () => logs(kept, by),
    issue: () => browse(ISSUES, by),
    back: () => history()?.goBack(),
    forward: () => history()?.goForward(),
    conversations: () => go("/"),
    timeline: pane("timeline"),
    code: pane("code"),
    backlog: pane("backlog"),
    roadmap: pane("roadmap"),
    share: pane("share"),
  });

  // Which of the Go menu's items can be pressed, as [`enabled`] answers it from
  // what the page last said and where the history stands — applied here and
  // decided there. Asked again whenever either moves, and once now, so that a
  // menu with no window behind it yet is a menu with nowhere to go.
  const regrey = (): void => {
    const now = history();
    grey(
      bar,
      enabled(going, {
        back: now?.canGoBack() ?? false,
        forward: now?.canGoForward() ?? false,
      }),
    );
  };
  regrey();

  try {
    const waited = await healthy(HEALTH);
    say(`the server answered after ${waited} ms`);
  } catch (trouble) {
    if (!(trouble instanceof NeverCameUp)) {
      throw trouble;
    }
    // Said and shown: there is no window yet, so the dialog stands on its
    // own, and the line is what a developer running `pnpm start` is reading.
    say(`the server never came up — ${trouble.message}`);
    silent(kept, trouble.waited);
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

  // And whether the Update the last run handed over to landed: a start on
  // another version than the one remembered is an install that failed
  // quietly, which this device's row is told as its trouble line — the server
  // that saw the press has gone, and this one is the first to hear of it.
  // Forgotten either way.
  const unlanded = missed(join(userData, HANDED_OVER), app.getVersion());
  if (unlanded !== undefined) {
    say(`the last run handed over to an Update that didn't land — ${unlanded}`);
    told(UPDATE_FAILED, data === undefined ? undefined : keyIn(data), { why: unlanded }).catch(
      (trouble: unknown) =>
        say(`the server could not be told the Update didn't land — ${String(trouble)}`),
    );
  }

  // What the icon is made of, made before there is a window for it to open:
  // **Show tray icon** can ask for it back at any moment the app is running, so
  // the raise below and the one [`enact`] makes are the same value. `onscreen`
  // rather than the window itself, for the reason `second-instance` reads it —
  // this is built before there is one.
  const trayed: Trayed = {
    icon: artwork(install),
    kept,
    by,
    open: () => {
      if (onscreen !== undefined) {
        forward(onscreen);
      }
    },
    quit: () => app.quit(),
    platform: machine.platform,
  };

  // The tray app's own registrations, taken over once — before anything below
  // reads this app's, because on a machine that had that app they are what the
  // human asked for and the API has never heard of either. The launch agent on a
  // Mac and the Run value on Windows: each is nothing at all off its own
  // platform, and nothing where there is none to find, which is every machine
  // from the version this ships in onwards.
  takeOverAgent(registering, login);
  takeOverValue(registering, login);

  // And the registration rewritten while there is one, which is what heals an
  // app that was moved: the entry a login reads names where the app used to be,
  // and a launch by hand is the moment that can be put right. A machine nobody
  // asked to be started on is left exactly as it is.
  starts.refresh();

  // The bridge's acts, enacted here because here is the process that can —
  // the file is read and written, the icon is raised and lowered, the log file
  // is handed to whatever the desktop reads text with, the platform is asked
  // about its startup registration and whether it shows notifications at all,
  // where this run is from is said and its Install pressed, and the window is
  // dressed to the page's own head. Registered before the window is opened, because the page is
  // loaded the moment there is one and a page that asked before this would be
  // asking nobody.
  ipcMain.handle(ASKED, () => settings(desk));
  ipcMain.handle(SET, (_event, sent: unknown) => enact(desk, trayed, sent));
  ipcMain.handle(LOGS, () => logs(kept, by));
  ipcMain.handle(STARTUP, () => starts.standing());
  ipcMain.handle(REGISTER, (_event, asked: unknown) => ticked(starts, asked));
  ipcMain.handle(NOTIFIABLE, () => Notification.isSupported());

  // And where this run is from, which is the reading the offer above made, and
  // Install pressed on the Desktop page, which raises the offer's own dialog
  // over the window it was pressed in.
  ipcMain.handle(PLACED, () => placement(here, machine));
  ipcMain.handle(INSTALLS, (event, stopping: unknown) =>
    pressedInstall(here, machine, desk, BrowserWindow.fromWebContents(event.sender), stopping),
  );

  // And the one that is a statement rather than a question, so `on` rather than
  // `handle`. The window is the one the push came *from* rather than `onscreen`:
  // this is registered before there is a window at all, and the page that says
  // what it is drawn in is by definition inside the window that is to be dressed
  // to it.
  ipcMain.on(HEAD, (event, pushed: unknown) => {
    wearing(BrowserWindow.fromWebContents(event.sender), machine.platform, pushed);
  });

  // And the other statement: what the open Conversation can go to, checked
  // before it is held — a push of the wrong shape is dropped and said rather
  // than thrown on — and held for the Go menu to read.
  ipcMain.on(WHERE, (_event, pushed: unknown) => {
    const heard = where(pushed);

    if (heard === undefined) {
      say("the page said where the open conversation can go in a shape this app does not read");
      return;
    }

    going = heard;
    say(`the open conversation can go to ${going.panes.map(({ pane }) => pane).join(", ") || "nothing"}`);
    regrey();
  });

  // Whether this launch is a login's — the flag the registration writes, or a
  // Mac saying its login item started this — read against the tray, because the
  // icon is the whole of what makes a hidden app reachable (ADR-0020). Read
  // before the window is opened, that being the one thing it decides.
  const atLogin = starts.atLogin();

  // Said whatever comes of it, because a Mac's login start is the one that
  // carries nothing on the command line: the log is the only place it shows
  // that the platform was asked and what it answered, and a window that came up
  // over somebody's login is read here first.
  if (atLogin) {
    say("macOS says its own login item is what started this run");
  }

  const unseen = hidden(process.argv, settings(desk), atLogin);
  if (unseen) {
    say("this launch is a login's and there is an icon in the tray, so no window comes up");
  }

  // The key is read at every load rather than once here: **Reset key** on the
  // phone writes that file while this window is open, and a link built from a
  // secret read at startup is a 401 with extra steps.
  const opened = open({
    origin: ORIGIN,
    secret: () => (data === undefined ? undefined : keyIn(data)),
    state: join(userData, FILE),

    // The bridge, beside the compiled main process rather than anywhere the
    // page could name: this directory is what `pnpm build` emits into, and the
    // file is the `.mjs` an ESM preload has to be — see `bridge.ts`.
    preload: join(import.meta.dirname, PRELOAD),

    // Read at the moment of the press, for the reason the key is: the settings
    // file is what the Desktop page writes, and a policy read once at startup
    // would be a radio nobody could see the effect of without a restart. A quit
    // already under way is not a press at all.
    closing: () => (quitting ? "quit" : closing(settings(desk), machine.platform)),

    // A login start with an icon to come back from opens no window over
    // whatever the human is doing — see [`hidden`](./startup.js), which is what
    // `--no-open` meant for the tray app.
    hidden: unseen,

    // The same answer the tray's **View Logs** is given: a link out of the
    // workbench and the log file are the two things this app hands to somebody
    // else's program, and one machine opens both.
    by,
  });
  const window = opened.window;
  onscreen = window;
  visiting = opened.visit;

  // And the history moving is Back and Forward greying: a document arriving,
  // and every route the page changes in place, which is most of them.
  window.webContents.on("did-navigate", regrey);
  window.webContents.on("did-navigate-in-page", regrey);

  // **And a Dock activation is the window coming back**, which is the other half
  // of what closing means on a Mac (ADR-0020): the app is a regular Dock app
  // now, so pressing its icon there is the same act as **Open** on the tray.
  // Registered everywhere, being a Mac's event to emit.
  //
  // **Registered here, after the window, rather than beside the handlers above
  // — and that is the whole of what keeps a hidden login start hidden.**
  // Electron emits `activate` when the application is activated, *including at
  // a first launch*, so a handler registered before `whenReady` catches the
  // launch's own activation and has nothing to bring forward yet. Remembered
  // for later, that is a window shown over somebody's login by the one arm
  // `hidden` below was written to prevent — the flag it would set is answered
  // unconditionally, so it overrules `hidden` rather than asking it. Registered
  // once there is a window, the launch's own activation is never seen and every
  // activation that is seen is somebody pressing the tile.
  app.on("activate", () => forward(window));

  // And then the icon, which is the other way to this window and the only one
  // while it is off the screen — which is why the close policy falls to Quit
  // without it. Shown unless this machine has said otherwise, which is where
  // the last run left the switch — moving it while the app is running is
  // [`enact`]'s, and it raises and lowers this same icon.
  if (settings(desk).trayIcon) {
    raise(trayed);
  } else {
    say("the desktop settings say no tray icon, so there is none — and closing the window quits");
  }

  // **And the notices stream, held for the rest of the run** — the phones' push
  // brought to this desk, which Web Push inside Electron cannot do (ADR-0020,
  // *Amended: notifications are the app's own*). Opened only now, once the
  // server has answered and there is a window for a click to bring forward, and
  // logged in with the key read afresh at every opening, as the window's link
  // is. Let go as the app quits, so that a drop at the end is not news.
  const listening = new AbortController();
  app.on("will-quit", () => listening.abort());
  void hold(
    {
      url: NOTICES,
      secret: () => (data === undefined ? undefined : keyIn(data)),
      // The switch read at every notice, as the close policy is read at every
      // press: a set is enacted in the run it is made in, and the stream is held
      // either way so that turning it back on needs nothing reopened.
      heard: (arrived) => noticed(opened, arrived, settings(desk).notifications),
      say,
    },
    listening.signal,
  );

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
