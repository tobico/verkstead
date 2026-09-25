//! The one window, opened on the login link.
//!
//! The second file at the edge, and the first one that draws anything: it holds
//! a `BrowserWindow`, so vitest cannot run it and the lint wall in
//! `eslint.config.js` names it. Everything it decides is somewhere else — where
//! the workbench is, what the key is, how a link is built — and what is left
//! here is the window and the two events it answers.
//!
//! **Loaded exactly as it is served.** Nothing about the viewer changes to draw
//! inside the app and nothing on the wire changes: this is the same document a
//! browser on this machine gets, over the same loopback origin.
//!
//! **A 401 on the window's own frame is the key having been reset from the
//! phone.** **Reset key** at the foot of Remote Access re-issues the secret, and
//! everything holding the old one meets a 401 on its next request. So the frame's
//! own navigation response is watched — not a subresource's, which is a different
//! failure with a different answer — the file is read again and the link is
//! loaded again. Once per navigation, so a key that is genuinely unreadable is a
//! window sitting on a refusal rather than a loop.
//!
//! The decorated window is this stage's; the frameless one with the controls
//! overlay, the remembered bounds and the links that leave for the system
//! browser are the stages after it.

import { BrowserWindow } from "electron";

import { link } from "./key.js";
import { say } from "./log.js";

/// What the window opens at on a machine that has not told it otherwise. A
/// desktop-sized workbench rather than a phone-sized one, and remembering where
/// the human put it afterwards is the window-manners task's.
const SIZE = { width: 1280, height: 860 };

/// What the gate answers anything that has not shown the current key.
const REFUSED = 401;

/// What opening the window needs, none of which it works out for itself.
export interface Workbench {
  /// The origin the window loads, which is the one the sidecar answers on.
  origin: string;

  /// The **Workbench Key** as it is on disk *now*, or `undefined` where there is
  /// none to read. Asked again at every load rather than once: that is what
  /// makes a reset from the phone a reload rather than a restart.
  secret: () => string | undefined;
}

/// Open the window on the workbench, logged in.
export function open(workbench: Workbench): BrowserWindow {
  const window = new BrowserWindow({
    ...SIZE,
    // What the window is called until the document says, which is the product
    // rather than the package this is built from.
    title: "Verkstead",
  });

  // Whether the load now on its way is already an answer to a refusal. Set when
  // one is made and cleared by any navigation that was not refused, so a
  // refusal has exactly one retry behind it however many times it is met.
  let recovering = false;

  const load = (): void => {
    const secret = workbench.secret();

    if (secret === undefined) {
      // The bare address, which the gate refuses — and that refusal is what
      // reads the file again, moments later. The one mechanism for both the key
      // that has been reset and the key that is not written yet.
      say(`no workbench key to read yet, so the window opens on ${workbench.origin} bare`);
    }

    const target = secret === undefined ? workbench.origin : link(workbench.origin, secret);

    // The origin rather than the link, here and everywhere: these lines go to a
    // file a menu item opens on somebody's desk, and a key written there is a
    // login for anybody reading over a shoulder (ADR-0015).
    window.webContents.loadURL(target).catch((trouble: unknown) => {
      say(`the window could not load ${workbench.origin} — ${String(trouble)}`);
    });
  };

  // A main frame navigation and its response code. `did-navigate` is the main
  // frame's alone — a subresource's 401 is something else, answered somewhere
  // else — and a navigation that was no HTTP request at all reports -1, which is
  // not a refusal either.
  window.webContents.on("did-navigate", (_event, _url, code) => {
    if (code !== REFUSED) {
      recovering = false;
      return;
    }

    if (recovering) {
      say("the workbench refused the key that was just read — the window is on a refusal");
      return;
    }

    recovering = true;
    say("the workbench refused the window's key, so it is read again");
    load();
  });

  say(`the window is opening on ${workbench.origin}`);
  load();

  return window;
}

/// Bring the window forward, which is what a second launch of the app comes to.
///
/// Electron's single-instance lock hands the second launch's argument over to
/// the first and ends it, so this runs in the app that is already here. Three
/// steps rather than a `focus`, because the window this reaches may be
/// minimised, may be hidden behind everything, and may — from stage 03, where
/// closing can mean keep running — not be on the screen at all.
export function forward(window: BrowserWindow): void {
  if (window.isMinimized()) {
    window.restore();
  }
  window.show();
  window.focus();
}
