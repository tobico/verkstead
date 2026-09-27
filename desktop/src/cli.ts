//! Where the headless `verkstead` is — two answers, and only one of them exists
//! yet.
//!
//! The app carries the CLI and starts it as its sidecar (ADR-0020), so finding
//! it is the first thing the app does. **In a checkout** it is the workspace's
//! own build under `target/`, which is what a developer has after `cargo
//! build` and what `pnpm start` runs against. **In a packed app** it is the CLI
//! inside the bundle, in a directory of its own rather than at the install root
//! — the root holds a launcher named for the product, and on Windows a
//! `verkstead` there would answer a `PATH` lookup with the window instead of
//! the guide. Stages 05 to 08 are what put a binary at that path; the case is
//! named here so that the app is not the thing that has to change when they do.
//!
//! **And an environment variable overrides both**, because a developer running
//! the app against a release build, or against the binary a Release shipped,
//! should not have to edit the app to do it.
//!
//! Nothing here touches the filesystem: the path is a function of where the app
//! is and what the environment says, and whether anything is at it is the
//! caller's question — which is what lets every arm of this be an ordinary
//! unit test.

import { join, resolve } from "node:path";

/// The environment variable that names a binary outright. Verkstead's own
/// prefix, because that is what every other variable this project reads has —
/// `VERKSTEAD_DATA_DIR`, `VERKSTEAD_LISTEN`.
export const OVERRIDE = "VERKSTEAD_CLI";

/// The directory a packed app keeps the CLI in, relative to its resources. Its
/// own rather than the root, for the reason at the top of this file.
const BUNDLED = "cli";

/// Where the app is running from, as the answer below is a function of.
///
/// A value rather than a reach into `electron`, so that all three arms are
/// exercised by tests that never start an application — see the wall in
/// `eslint.config.js`.
export interface Install {
  /// Whether this is a packed app rather than a checkout: `app.isPackaged`.
  packaged: boolean;
  /// The directory the main process was loaded from — `desktop/dist` in a
  /// checkout, which is two levels under the repository root.
  entry: string;
  /// Where a packed app keeps what it ships beside the code:
  /// `process.resourcesPath`.
  resources: string;
  /// Which platform's file name the binary has.
  platform: NodeJS.Platform;
  /// The process environment, read once by the caller.
  env: Partial<Record<string, string>>;
}

/// What the binary is called here.
function named(platform: NodeJS.Platform): string {
  return platform === "win32" ? "verkstead.exe" : "verkstead";
}

/// The path the app will start its sidecar from.
export function cli(install: Install): string {
  const said = install.env[OVERRIDE];
  if (said !== undefined && said !== "") {
    // Resolved against nothing: a variable naming a relative path means it
    // relative to wherever the app was started, which is the shell's own
    // notion and the only one a developer typing it has.
    return resolve(said);
  }

  const name = named(install.platform);

  if (install.packaged) {
    return join(install.resources, BUNDLED, name);
  }

  // `desktop/dist` → the repository root → the workspace's debug build, which
  // is what `cargo build` leaves and what the development docs tell a
  // developer to make. A release build is the variable above.
  return resolve(install.entry, "..", "..", "target", "debug", name);
}
