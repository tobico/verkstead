//! The icon in the system tray: Open, View Logs and Quit.
//!
//! The third file at the edge, and it is here for the reason `window.ts` is: a
//! `Tray` is a thing rather than a value — an icon on somebody's panel, which
//! nothing can be handed in — so this module holds one and everything it
//! *decides* is [`chosen.ts`](./chosen.js)'s, which vitest runs. See the wall
//! at the top of `eslint.config.js`.
//!
//! **What the tray is for is the two things the window cannot do for itself**:
//! put itself back in front of the human once it is gone from the screen, and
//! stop the app. The Rust tray app was those two and nothing else (ADR-0012);
//! this one adds View Logs, because a log file nobody can find is a log file
//! nobody sends.
//!
//! **Open is the window brought forward**, the same three steps a second launch
//! of the app comes to — see [`forward`](./window.js), whose two callers this is
//! the other of. Minimised, buried, and from the close policy that arrives next,
//! not on the screen at all: the icon answers all three.
//!
//! **Quit never asks.** Somebody who picked Quit has said what they meant, and
//! the warning the close policy brings is the close button's alone (ADR-0020).
//! What it takes with it is the sidecar, which the app's own quit handling is
//! already what does.
//!
//! **A click is the panel's own choice of gesture.** Electron emits its click on
//! the StatusNotifierItem activation, and the specification does not say what
//! causes one: some panels send it on a left click, some on a double click, and
//! some only open the menu. So the click is wired *and* Open is the first item
//! on the menu — which is what the Rust app relied on for the same reason, and
//! what makes the icon and its menu mean one thing rather than two.
//!
//! **And a desktop with no tray host needs no guard.** The pinned Electron
//! constructs a tray with no StatusNotifierWatcher on the session bus, takes a
//! context menu and is not destroyed, so nothing here has to survive a throw —
//! and the icon appearing when a panel arrives later is stage 03's last task to
//! prove on a real one.

import { dialog, Menu, shell, Tray } from "electron";

import { type Chosen, label, MENU, viewing } from "./chosen.js";
import { type Kept, say } from "./log.js";

/// What the tray needs, none of which it works out for itself.
export interface Trayed {
  /// The artwork's path — [`artwork`](./artwork.js)'s answer.
  icon: string;

  /// Where this run's logging went, which is the whole of what **View Logs**
  /// is about. Held from the moment the log file was opened, because by the
  /// time somebody picks the item there is nothing left to ask.
  kept: Kept;

  /// Put the window back in front of the human.
  open: () => void;

  /// Stop the app, and the sidecar with it.
  quit: () => void;
}

/// The icon itself, kept for as long as there is an app to have one.
///
/// **A `Tray` nothing holds is an icon that can go**: the object *is* the icon,
/// so a garbage collection that took the last reference to it would take the
/// icon off the panel. Held here rather than handed back because the one thing
/// anybody does with it is [`lower`] below — and because **Show tray icon**,
/// when it arrives, is this pair of functions and nothing else.
let icon: Tray | undefined;

/// Put the icon in the tray, with its menu on it and its click wired.
export function raise(trayed: Trayed): void {
  const tray = new Tray(trayed.icon);

  // What a panel shows when the pointer rests on the icon, and on some desktops
  // what the accessibility layer reads out. The product's name, which is all
  // there is to say about an icon that is the whole of Verkstead on that panel.
  tray.setToolTip("Verkstead");

  // A record rather than a switch, so that an item added to the menu in
  // `chosen.ts` is a compile error here until it has been given something to
  // do — the menu being a value is what buys that.
  const acts: Record<Chosen, () => void> = {
    open: trayed.open,
    logs: () => logs(trayed.kept),
    quit: trayed.quit,
  };

  tray.setContextMenu(
    Menu.buildFromTemplate(
      MENU.map((chosen) => ({ id: chosen, label: label(chosen), click: acts[chosen] })),
    ),
  );

  tray.on("click", trayed.open);

  icon = tray;
  say("the icon is in the tray");
}

/// Take the icon off the panel.
///
/// Called as the app goes, and first of the things it does then: the icon is
/// the one piece of this app that is on somebody else's window, stopping the
/// sidecar can take a moment, and an icon still sitting there after Quit was
/// picked is an app that looks stuck. `crates/desktop/src/tray.rs` let go of
/// its own for the same reason.
export function lower(): void {
  icon?.destroy();
  icon = undefined;
}

/// **View Logs**: this run's log file, or why there is not one.
function logs(kept: Kept): void {
  const act = viewing(kept);

  if (!("open" in act)) {
    // Said as a remark rather than as an error: the app is running, and it was
    // asked for something it happens not to have. On the screen because the
    // lines this run is writing are going to a standard error that an app
    // started from an icon has nobody reading.
    say("View Logs was picked, and this run has no log file to open");

    void dialog.showMessageBox({
      type: "info",
      title: "Verkstead",
      message: "There is no log file to open",
      detail: act.note,
    });
    return;
  }

  say(`View Logs opens ${act.open}`);

  // Handed over rather than waited on, the same reading the Rust app made of
  // it in `opener.rs`: what starts is somebody else's program, and a text editor
  // that takes ten seconds to come up is not something the app should be
  // sitting on. `openPath` reports a refusal as a string rather than as a
  // rejection, so both endings are read.
  shell
    .openPath(act.open)
    .then((trouble) => {
      if (trouble !== "") {
        say(`the log file could not be opened — ${trouble}`);
      }
    })
    .catch((trouble: unknown) => {
      say(`the log file could not be opened — ${String(trouble)}`);
    });
}
