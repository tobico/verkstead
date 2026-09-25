//! Where the tray's icon is — one question, asked in one place.
//!
//! The Rust tray app put the artwork *in* the binary, because that binary ships
//! as one file and an icon read off disk is an icon that can go missing. This
//! app is a directory either way, so the icon is a file either way, and what
//! changes between a checkout and a packed app is only where that file is: the
//! repository's own `assets/icons` while a developer is running `pnpm start`,
//! and what the packer put beside the code once there is a packed app. Stage 05
//! is what puts it there; the case is named here so that the app is not the
//! thing that has to change when it does — the same shape, and for the same
//! reason, as [`cli.ts`](./cli.js) beside it.
//!
//! **The 192px one of the generated set**, which is what
//! `crates/desktop/src/tray.rs` drew and for its reason: the panel picks the
//! height — around 22 points on most, twice that on a HiDPI one — and scales
//! what it is given to it, and an icon scaled *up* is the one that looks wrong.
//!
//! Nothing here touches the filesystem either: the path is a function of where
//! the app is, and every arm of it is an ordinary unit test.

import { join, resolve } from "node:path";

import type { Install } from "./cli.js";

/// The file, in `assets/icons` and beside the packed app's code alike — one
/// name for both, so that the packer and the app are saying the same word.
export const ARTWORK = "icon-192.png";

/// The path the tray draws its icon from.
export function artwork(install: Install): string {
  if (install.packaged) {
    return join(install.resources, ARTWORK);
  }

  // `desktop/dist` → the repository root → the generated icons, which is where
  // `tools/generate-icons.sh` leaves them and where the Rust app read the same
  // file from.
  return resolve(install.entry, "..", "..", "assets", "icons", ARTWORK);
}
