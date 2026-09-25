//! What the tray's menu offers, and what picking one of them means.
//!
//! The tray itself is a thing — an icon on somebody's panel — and it is
//! [`tray.ts`](./tray.js)'s, on the far side of the wall in `eslint.config.js`.
//! What is here is the half that is a value: which items there are, in which
//! order, what each one says, and the one of the three whose meaning is a
//! question rather than a call — **View Logs**, which does one thing where this
//! run has a log file and another where it has not.
//!
//! **The order is the menu's meaning as much as the labels are.** A panel that
//! reports no click of its own opens the menu instead, so the first item is
//! what the icon means by default — which is why Open is first, exactly as
//! `crates/desktop/src/tray.rs` had it and for the same reason. Quit is last
//! for the reverse of it, and View Logs is what is neither.
//!
//! The Rust app's menu had a fourth item between them, Launch on Startup, and
//! this one does not: here that is a checkbox on the Desktop page instead
//! (ADR-0020), so the tray is three items and the page is where the settings
//! are.

import type { Kept } from "./log.js";

/// The menu, in the order it is drawn — and each entry is the id its item
/// carries, so a pick is read back as the thing that was picked.
export const MENU = ["open", "logs", "quit"] as const;

/// One of the three.
export type Chosen = (typeof MENU)[number];

/// What the item says.
export function label(chosen: Chosen): string {
  switch (chosen) {
    case "open":
      return "Open";
    case "logs":
      return "View Logs";
    case "quit":
      return "Quit";
  }
}

/// What **View Logs** does about where this run's logging went.
export type Viewing =
  /// Hand this file to whatever the desktop reads text with.
  | { readonly open: string }
  /// Say this instead. There is no file, and the reason there is none is the
  /// only thing the item has to offer — said on the screen, because the log it
  /// would otherwise be written into is the very thing this machine has not
  /// got.
  | { readonly note: string };

/// Read where the logging went into what the menu item does about it.
///
/// The whole of the item's own thinking, which is why it is here rather than
/// inside the tray: a machine with nowhere to keep a log file has only lost the
/// log, so the item says so rather than failing silently or opening nothing.
export function viewing(kept: Kept): Viewing {
  return "file" in kept ? { open: kept.file } : { note: kept.nowhere };
}
