//! What a press of the window's close button comes to.
//!
//! Three answers — hide the window and leave the app running, ask first, or
//! quit — out of three things: the position the human chose, whether there is
//! an icon in the tray, and which platform this is. A function of values, so
//! the Mac arm is an ordinary unit test on Linux; enacting it is the window's,
//! in [`window.ts`](./window.js).
//!
//! **With the tray off the choice falls to Quit** (ADR-0020), whatever the
//! settings file says, and deliberately not to *ask first*: an app with no icon
//! that has hidden its only window is a Verkstead nobody can reach, with no way
//! back to it but the command line. So the setting is not overridden — it is
//! still there, and turning the tray back on restores it — but while there is
//! no icon, the only close this app performs is the one that leaves nothing
//! behind. The greyed position on the page and the note saying why are how the
//! human is told; this is what enacts it.
//!
//! **A Mac is the platform's own**, and the radio is not consulted there at
//! all. Closing a window on a Mac leaves the application running in the Dock
//! and a Dock activation brings it back — that is what every Mac app does, and
//! the app is a regular Dock app now rather than the menu-bar-only thing
//! ADR-0012's tray asked for. Cmd+Q is what quits, as it is everywhere on that
//! platform.
//!
//! **And the warning is the close button's alone** (ADR-0020). The tray's Quit
//! and the menu's Quit are not closes and never reach this: somebody who picked
//! Quit has already said what they meant, and being asked again would be the
//! app arguing with them.

import type { Settings } from "./settings.js";

/// What to do about a close.
export type Closing =
  /// Take the window off the screen and leave the app — and the sidecar —
  /// running. The window is kept rather than destroyed, so Open on the tray is
  /// the same window coming back.
  | "hide"
  /// Put the warning up first, and do what it is answered with.
  | "ask"
  /// Let the close happen, which ends the app.
  | "quit";

/// What this close means, given what was chosen, whether there is an icon to
/// go back to, and whose platform this is.
export function closing(chosen: Settings, platform: NodeJS.Platform): Closing {
  // The platform's own answer, and it is not a position on the radio: the page
  // does not draw one on a Mac, and a settings file carried over from another
  // machine does not change what closing a window means there.
  if (platform === "darwin") {
    return "hide";
  }

  // No icon, no way back to a hidden window.
  if (!chosen.trayIcon) {
    return "quit";
  }

  switch (chosen.whenClosed) {
    case "tray":
      return "hide";
    case "ask":
      return "ask";
    case "quit":
      return "quit";
  }
}

/// What the warning says, which is the one thing about this close that is worth
/// stopping somebody over: a Verkstead that quits takes its sessions with it,
/// and a session stopped halfway through is work somebody was in the middle of.
///
/// Here rather than in the window because the words are a value — this is what
/// the dialog is built from, and the buttons are in the order the platform's
/// own dialogs put the action and the way out in.
export const WARNING = {
  /// The question, which is what a native dialog draws large.
  message: "Quit Verkstead?",

  /// And what it costs, which is the reason there is a dialog at all.
  detail: "Any sessions that are running will be stopped.",

  /// The way on, and the way out. Cancel is what the window's own close button
  /// and the Escape key come to, so it is the one that changes nothing.
  buttons: ["Quit", "Cancel"] as const,
};

/// Which of [`WARNING`]'s buttons means quit.
export const QUIT = 0;

/// And which means leave everything as it was.
export const CANCEL = 1;
