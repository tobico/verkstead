//! Reading and deleting one value of the Windows registry, which is the one
//! thing **Launch on Startup** needs and nothing this app has an API for.
//!
//! Electron has none, this app has no dependencies at all and is not the place to
//! acquire a native one — so it is `reg.exe` run and its exit status read,
//! injected as [`Registry`] the way the login-item API is, which is what makes
//! every arm an ordinary vitest on the Linux runner.
//!
//! **Two callers, and the Run key is the whole of what either asks about.**
//! [`runkey.ts`](./runkey.js) reads the tray app's own value there and deletes
//! it; [`startup.ts`](./startup.js) reads this app's own, because Electron's
//! account of the Run key lists only the values that name the executable that is
//! running — so a registration left behind by an app that has moved is one the
//! API cannot see and the whole point of the rewrite at a launch.
//!
//! **A value that is not there, a key that is not there and a command that would
//! not run are one answer**, which is the reading every arm of that file gives an
//! unreadable registration: nothing is registered here.

import { spawnSync } from "node:child_process";

/// The key Verkstead's registration goes in, as `reg.exe` names it: what Windows
/// starts at sign-in, under the current user.
///
/// **The current user's own rather than the machine's**, which is the key the
/// tray app wrote and the key Electron's API writes — the box is about this
/// human's sign-in, and `HKEY_LOCAL_MACHINE` would be every account on the
/// machine and an elevation nobody asked for.
export const RUN = String.raw`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`;

/// Reading and deleting one value, which is the whole of what this app does to a
/// registry.
export interface Registry {
  /// Whether `value` is there under `key`, whatever it holds.
  ///
  /// **One answer for every way of not finding it**: no value, no key and no
  /// program to ask with are all `false`, because what this is asked is whether
  /// there is a registration there.
  has(key: string, value: string): boolean;

  /// Take `value` away, and throw where it could not be taken.
  ///
  /// Thrown rather than answered, because the caller has something to do about
  /// it — see [`takeOver`](./runkey.js).
  remove(key: string, value: string): void;
}

/// `reg.exe`, which is what Windows has where this would have had an API.
///
/// Shipped with the platform since it had a registry, so there is nothing to look
/// for on the `PATH` and nothing to fall back to — and a machine that somehow
/// cannot run it reads as a machine with nothing registered, which is what the
/// contract above says.
export const reg: Registry = {
  has: (key, value) => ran(["query", key, "/v", value]) === 0,

  remove: (key, value) => {
    const status = ran(["delete", key, "/v", value, "/f"]);

    if (status !== 0) {
      throw new Error(`reg delete ${key} /v ${value} ended with ${String(status)}`);
    }
  },
};

/// Run `reg.exe` with `args` and say how it ended, or `undefined` where it could
/// not be run at all.
///
/// Nothing of its output is read and nothing of it is shown: the exit status is
/// the whole of the answer either way, and a console window flashing up at the
/// launch of a windowed app is not something a human should have to see.
function ran(args: string[]): number | undefined {
  const done = spawnSync("reg.exe", args, {
    stdio: ["ignore", "ignore", "ignore"],
    windowsHide: true,
  });

  return done.status ?? undefined;
}
