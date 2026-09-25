//! Where a directory of Verkstead's own is when nobody has said — which the app
//! has to answer the same way the server does.
//!
//! The app reads the **Workbench Key** out of the **Data Directory** the sidecar
//! was started against, and it was started against nothing: the child inherits
//! this process's environment, so what the server resolves is what this
//! resolves. Two answers that disagreed would be an app reading a file no
//! server ever wrote.
//!
//! So this is `crates/server/src/platform.rs`'s own reading, arm for arm:
//! `$XDG_DATA_HOME` or `~/.local/share` on Linux, `~/Library/Application
//! Support` on macOS, `%APPDATA%` on Windows, and the name lowercase where the
//! platform's neighbours are named for their programs and capitalised where they
//! are named for their products.
//!
//! **A function of the values rather than of the process**, for the reason that
//! module gives: the platform and the environment go in and a path comes back,
//! so all three arms are exercised by ordinary unit tests on Linux. The one read
//! of the real environment is at the edge, in `main.ts`.
//!
//! The **Log Directory** is the other directory resolved this way, and it is
//! [`logDir`] below — the same three arms, read out of the same values.

import { join } from "node:path";

/// The name the directory takes where the platform's convention is the binary's
/// own name in lowercase — Linux, where it stands among the `~/.local/share`
/// neighbours that are named for their programs.
const LOWERCASE = "verkstead";

/// And where the convention is the product's name as a human writes it — macOS
/// and Windows, whose `Application Support` and `%APPDATA%` hold the names
/// people read in a file dialog.
const CAPITALISED = "Verkstead";

/// The variable that says where the Data Directory is. The server's own — see
/// `verkstead_server::Config` — and the whole of how a checkout run reaches a
/// checkout's data, the app having no flag of its own for it.
export const SAID = "VERKSTEAD_DATA_DIR";

/// Whose conventions a directory is resolved by, and out of what.
///
/// Values rather than reads of `process`, so that the arm this machine will
/// never run is still an arm a test can call.
export interface Machine {
  /// Which platform's conventions to read by: `process.platform`.
  platform: NodeJS.Platform;
  /// The process environment, read once by the caller.
  env: Partial<Record<string, string>>;
}

/// The Data Directory this run reads the key out of: what the environment said,
/// and the platform's own place for one otherwise.
///
/// `undefined` where the machine names nowhere to put one — a Linux box with no
/// absolute `HOME` and no `XDG_DATA_HOME`, a Windows one with no `%APPDATA%`.
/// The server refuses to start over that and so never answers health, so the app
/// has nothing to do about it but say so: a Verkstead with nowhere for a Data
/// Directory has nothing to serve.
export function dataDir(machine: Machine): string | undefined {
  const said = machine.env[SAID];

  // Taken as it was said, unresolved and unjudged. The sidecar is handed this
  // same variable and started with this process's own working directory, so a
  // relative value names one directory for both of them — which is what
  // `--data-dir .` out of a checkout has always meant. An empty variable is one
  // a shell exported without a value and says nothing at all.
  if (said !== undefined && said !== "") {
    return said;
  }

  return platformsOwn(machine);
}

/// Where the Data Directory goes on this platform, out of this environment.
function platformsOwn({ platform, env }: Machine): string | undefined {
  switch (platform) {
    case "darwin": {
      const home = absolute(env.HOME);
      return home === undefined
        ? undefined
        : join(home, "Library", "Application Support", CAPITALISED);
    }

    case "win32": {
      // The roaming application data, because everything in this directory is
      // the human's own and meant to follow them between machines. The Log
      // Directory is the local one, for the opposite reason.
      const appdata = set(env.APPDATA);
      return appdata === undefined ? undefined : join(appdata, CAPITALISED);
    }

    // Everything else is read the XDG way, which is what the server's own
    // `Platform::HERE` does with a Unix that is not a Mac.
    default: {
      const base = xdg(env.XDG_DATA_HOME, env.HOME, join(".local", "share"));
      return base === undefined ? undefined : join(base, LOWERCASE);
    }
  }
}

/// The XDG reading a Unix directory gets: the variable where it is set to an
/// absolute path, and `$HOME/<under>` where it is not — the specification's own
/// fallback, and what most machines have.
function xdg(variable: string | undefined, home: string | undefined, under: string): string | undefined {
  const said = absolute(variable);
  if (said !== undefined) {
    return said;
  }

  const fallback = absolute(home);
  return fallback === undefined ? undefined : join(fallback, under);
}

/// `value` where it is an absolute Unix path, and nothing where it is relative
/// or empty — as the XDG specification says of its own variables, and for the
/// reason this module exists: a directory resolved against wherever the app
/// happened to be started is the thing the platform default replaces.
///
/// Asked as a leading `/` rather than as `isAbsolute`, which is the same
/// question of the two Unixes this is for and the only one of the two that
/// keeps its answer where the arm is run somewhere else. `isAbsolute` answers
/// by the host's rules, so `/home/you` put through it on a Windows runner comes
/// back relative and a Linux arm this module exists to test resolves to
/// nowhere. See [`set`], which is the same crossing read from the Windows side.
///
/// **Exported, because it is not only these directories' question.** The
/// autostart directory a **Launch on Startup** entry goes in and the
/// `$APPIMAGE` that entry names in preference to the running executable are read
/// by the same rule, out of the same environment — see
/// [`startup.ts`](./startup.js). Not one of Verkstead's own directories either
/// way, which is why that module resolves its own rather than asking here for a
/// path.
export function absolute(value: string | undefined): string | undefined {
  return value !== undefined && value.startsWith("/") ? value : undefined;
}

/// `value` where the machine set it to anything at all.
///
/// What the Windows arm has instead of [`absolute`], deliberately: there is no
/// reading of `C:\Users\you\AppData\Roaming` that says the same thing on every
/// host this arm is tested on, and a `%APPDATA%` the platform set is absolute
/// by being what the platform set.
function set(value: string | undefined): string | undefined {
  return value !== undefined && value !== "" ? value : undefined;
}

/// The Log Directory this run writes its log file in, or `undefined` where the
/// machine names nowhere to put one.
///
/// `crates/server/src/platform.rs`'s `default_log_dir`, arm for arm, and the
/// three arms disagree about what this directory even *is*: a state directory
/// on Linux, a logs directory on macOS, the local rather than the roaming
/// application data on Windows — local because a log file follows nobody
/// between machines, which is the opposite of what the Data Directory holds.
///
/// **Nothing says otherwise.** There is no variable for this the way
/// [`SAID`] is one for the Data Directory: the server resolves it and
/// deliberately does not create it, and the app is what makes it — see
/// [`keep`](./log.js), which is also what answers the `undefined`.
export function logDir({ platform, env }: Machine): string | undefined {
  switch (platform) {
    case "darwin": {
      const home = absolute(env.HOME);
      return home === undefined ? undefined : join(home, "Library", "Logs", CAPITALISED);
    }

    case "win32": {
      const local = set(env.LOCALAPPDATA);
      return local === undefined ? undefined : join(local, CAPITALISED);
    }

    default: {
      // The state directory rather than the data one, and it is the same
      // specification saying both — so a relative `XDG_STATE_HOME` is ignored
      // here for the reason a relative `XDG_DATA_HOME` is there.
      const base = xdg(env.XDG_STATE_HOME, env.HOME, join(".local", "state"));
      return base === undefined ? undefined : join(base, LOWERCASE);
    }
  }
}
