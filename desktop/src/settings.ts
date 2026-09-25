//! The app's own settings: what closing the window means, and whether there is
//! an icon in the tray.
//!
//! **A file of the app's own, beside the one the window's place is kept in**
//! ([`bounds.ts`](./bounds.js)), and for the same reason (ADR-0020). Whether
//! this machine's Verkstead keeps running when its window is closed is a fact
//! about the desk in front of the human rather than anything the server was
//! told: the same account read from a laptop and from a desktop wants two
//! different answers, and neither of them is one `config.yaml` has an opinion
//! on. So nothing here goes on the wire — the split the per-device push switch
//! already made.
//!
//! **Two settings, and the shape is the whole of what the page and the bridge
//! read**: `whenClosed`, which is one of three positions, and `trayIcon`. The
//! defaults are keep running in the tray and an icon shown, which is the app
//! the human who has never opened the Desktop page is using.
//!
//! **A file that is not what this wrote reads as the defaults one setting at a
//! time.** Not all-or-nothing: a hand that edited the file and mistyped one
//! line has said nothing about the other, and a reading that threw both away
//! would turn a typo into a second surprise. The same tolerance the remembered
//! bounds and the viewer's own per-device storage both have, and it is why
//! each value is read on its own below rather than the object being checked as
//! a whole.
//!
//! **And what may be written to it is here too**, in [`changed`]: a set
//! arriving over the bridge from the page is checked against the shape before
//! it reaches the file. That reading is this module's rather than the bridge's
//! because the shape is this module's — the same three positions the radio
//! draws and the file is read by, checked in one place.
//!
//! What a close *comes to*, given these two and the platform, is
//! [`closing.ts`](./closing.js)'s: this module is the file, what is in it, and
//! what may go into it.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

import { say } from "./log.js";

/// What the settings file is called inside the app's user data. Named for what
/// it holds — the desktop's own settings — rather than for the app, which the
/// directory around it is already named for.
export const FILE = "desktop.json";

/// The positions of **When the window is closed**, in the order the radio draws
/// them: keep running in the tray, ask before quitting, quit.
///
/// A list rather than a union written out, so that the page's radio and the
/// bridge's validation are both reading the same three words this file is.
export const POSITIONS = ["tray", "ask", "quit"] as const;

/// One of the three.
export type WhenClosed = (typeof POSITIONS)[number];

/// Everything the app keeps about this desk.
export interface Settings {
  /// What a press of the window's close button means.
  whenClosed: WhenClosed;

  /// Whether there is an icon in the tray — and so, whether the app can be
  /// reached at all once its window is off the screen.
  trayIcon: boolean;
}

/// The app somebody who has never opened the Desktop page is using: it keeps
/// running in the tray, and there is a tray to keep running in.
export const DEFAULTS: Settings = { whenClosed: "tray", trayIcon: true };

/// What this machine's settings are, out of `file`.
///
/// Never throws and never refuses: a file that is missing is a machine that has
/// not been told, and one that is unreadable is a machine that has been told
/// something this cannot make out. Both are the defaults, which is the app
/// every first run gets.
export function settings(file: string): Settings {
  let written: string;

  try {
    written = readFileSync(file, "utf8");
  } catch {
    // The ordinary first run: nothing has been set, so nothing is there.
    return { ...DEFAULTS };
  }

  let read: unknown;
  try {
    read = JSON.parse(written);
  } catch {
    say(`the desktop settings in ${file} are not readable, so the app's own defaults are used`);
    return { ...DEFAULTS };
  }

  if (typeof read !== "object" || read === null) {
    say(`the desktop settings in ${file} are not a set of settings, so the defaults are used`);
    return { ...DEFAULTS };
  }

  const held = read as Record<string, unknown>;

  return {
    whenClosed: position(held.whenClosed) ?? DEFAULTS.whenClosed,
    trayIcon: typeof held.trayIcon === "boolean" ? held.trayIcon : DEFAULTS.trayIcon,
  };
}

/// `value` where it is one of the three positions, and nothing otherwise.
export function position(value: unknown): WhenClosed | undefined {
  return POSITIONS.find((held) => held === value);
}

/// What a set that arrived over the bridge asks for, where what it sent is
/// settings — and nothing where it is not.
///
/// **Because a renderer is a renderer.** The page is the app's own and the
/// window never leaves the one origin, but what comes up the bridge is checked
/// against the shape before it reaches the file all the same: a value that is
/// not one of the positions or not a boolean is refused rather than written,
/// and so is a key this app has no setting for.
///
/// **All or nothing, which is the opposite of what [`settings`] does with a
/// file** — and deliberately. A file that is half wrong is a human who
/// hand-edited one line and said nothing about the other, so the good half
/// stands; a set that is half wrong is a program with a bug in it, and writing
/// the half of it that parsed would be the app guessing at what a bug meant.
///
/// A change that names nothing is nothing to write, so it is refused with the
/// rest: every set this app makes comes from a control somebody moved.
export function changed(sent: unknown): Partial<Settings> | undefined {
  if (typeof sent !== "object" || sent === null || Array.isArray(sent)) {
    return undefined;
  }

  const asked = sent as Record<string, unknown>;
  const wanted: Partial<Settings> = {};

  for (const [key, value] of Object.entries(asked)) {
    switch (key) {
      case "whenClosed": {
        const chosen = position(value);
        if (chosen === undefined) {
          return undefined;
        }
        wanted.whenClosed = chosen;
        break;
      }
      case "trayIcon":
        if (typeof value !== "boolean") {
          return undefined;
        }
        wanted.trayIcon = value;
        break;
      default:
        return undefined;
    }
  }

  return Object.keys(wanted).length > 0 ? wanted : undefined;
}

/// Keep `chosen` for this run and the ones after it.
///
/// Never throws, for [`remember`](./bounds.js)'s reason: a settings file that
/// cannot be written is an app that goes on behaving the way it was behaving,
/// which is a smaller thing than an app that fell over while a checkbox was
/// pressed. The directory is made because a first run's user data may not exist
/// yet.
export function set(file: string, chosen: Settings): void {
  try {
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(file, `${JSON.stringify(chosen, null, 2)}\n`, "utf8");
  } catch (trouble) {
    say(`the desktop settings could not be kept in ${file} — ${String(trouble)}`);
  }
}
