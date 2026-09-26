//! Where the tray's icon is — one question, asked in one place.
//!
//! The Rust tray app put the artwork *in* the binary, because that binary ships
//! as one file and an icon read off disk is an icon that can go missing. This
//! app is a directory either way, so the icon is a file either way, and what
//! changes between a checkout and a packed app is only where that file is: the
//! repository's own artwork while a developer is running `pnpm start`, and what
//! the packer put beside the code once there is a packed app — the same shape,
//! and for the same reason, as [`cli.ts`](./cli.js) beside it.
//!
//! **A panel gets the 192px one of the generated set**, which is what
//! `crates/desktop/src/tray.rs` drew and for its reason: the panel picks the
//! height — around 22 points on most, twice that on a HiDPI one — and scales
//! what it is given to it, and an icon scaled *up* is the one that looks wrong.
//!
//! **A menu bar gets a file of its own, because a Mac does not scale it**
//! (stage 06). AppKit lays a status item out at the size of the image it is
//! handed and clips it to the bar, so the 192px icon came out on a real Mac as
//! two hundred points of hammer with its middle showing. What that platform is
//! given instead is the template image `tools/generate-packaging.sh` writes
//! beside the launcher icons: the drawing's own silhouette, at the 22 points the
//! bar lays out, with the `@2x` beside it for a Retina one. Template, so the
//! system draws it black on a light bar and white on a dark one — a full-colour
//! icon at that size is a blob on both.
//!
//! **And `Template` at the end of the name is the whole of how that is asked
//! for.** `nativeImage.createFromPath` reads the suffix off the file name, and
//! finds the `@2x` in the directory the path names — so what is handed over here
//! is the one-times file, and the name it carries is not a detail this module
//! may tidy.
//!
//! Nothing here touches the filesystem either: the path is a function of where
//! the app is running from and whose platform it is, and every arm of it is an
//! ordinary unit test.

import { join, resolve } from "node:path";

import type { Install } from "./cli.js";

/// The file a panel is given, in `assets/icons` and beside the packed app's code
/// alike — one name for both, so that the packer and the app are saying the same
/// word.
export const ARTWORK = "icon-192.png";

/// And the file a menu bar is given, in `packaging/` and beside the packed app's
/// code alike. Its `@2x` is read by the platform rather than named here.
export const TEMPLATE = "menubarTemplate.png";

/// The path the tray draws its icon from.
export function artwork(install: Install): string {
  // A Mac's menu bar is the one that wants the template; everything else is a
  // panel, and a panel scales the big one down.
  const file = install.platform === "darwin" ? TEMPLATE : ARTWORK;

  if (install.packaged) {
    return join(install.resources, file);
  }

  // `desktop/dist` → the repository root → the generated artwork, which is where
  // `tools/generate-icons.sh` and `tools/generate-packaging.sh` leave their own
  // and where the Rust app read the panel's from.
  return install.platform === "darwin"
    ? resolve(install.entry, "..", "..", "packaging", TEMPLATE)
    : resolve(install.entry, "..", "..", "assets", "icons", ARTWORK);
}
