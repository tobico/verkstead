//! `pnpm start` — the app, run on the dev shell's Electron.
//!
//! **Why this is a script and not `electron .`.** The `electron` package is
//! pinned here for its TypeScript definitions and for what electron-builder
//! will pack, and its install script — the one that downloads a couple of
//! hundred megabytes of runtime — is denied in `pnpm-workspace.yaml` (see
//! there). But the package still puts an `electron` command in
//! `node_modules/.bin`, and `pnpm run` puts that directory first on the
//! `PATH`, so a plain `electron .` finds the shim over the binary that is
//! actually installed and fails asking to be downloaded. So the search skips
//! anything under a `node_modules`, which leaves the dev shell's Electron and
//! nothing else — which is what ADR-0020 says a developer runs.
//!
//! Plain JavaScript, and outside `src/`: it is what runs the app rather than
//! part of it, and a thing that had to be compiled before it could start the
//! compiler's output would be a knot for no reason.

import { spawn } from "node:child_process";
import { accessSync, constants } from "node:fs";
import { delimiter, join, sep } from "node:path";

/// What Electron is called, in the order a shell would take them.
const NAMES = process.platform === "win32" ? ["electron.cmd", "electron.exe"] : ["electron"];

/// The first Electron on the `PATH` that is not a package's own shim.
function electron() {
  for (const directory of (process.env.PATH ?? "").split(delimiter)) {
    if (directory === "" || directory.split(sep).includes("node_modules")) {
      continue;
    }
    for (const name of NAMES) {
      const candidate = join(directory, name);
      try {
        accessSync(candidate, constants.X_OK);
        return candidate;
      } catch {
        // Not here.
      }
    }
  }
  return undefined;
}

const found = electron();

if (found === undefined) {
  process.stderr.write(
    "No Electron on the PATH. `pnpm start` runs the dev shell's Electron — " +
      "see `flake.nix` — so this wants a `nix develop` around it.\n",
  );
  process.exit(1);
}

// The app is this directory, and every argument after `pnpm start --` is the
// app's own. `inherit`, so that what Electron itself has to say lands where
// the developer is reading. The app's own lines and the sidecar's go to
// `verkstead.log` under the Log Directory instead — the app names that file on
// this same stream as it opens it, which is the one line that goes both ways.
const app = spawn(found, [".", ...process.argv.slice(2)], { stdio: "inherit" });
app.on("exit", (code, signal) => process.exit(signal !== null ? 1 : (code ?? 0)));
