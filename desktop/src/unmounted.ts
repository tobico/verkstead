//! What the sidecar is handed: this process's environment, with the AppImage's
//! own doing taken back out of it.
//!
//! **A packed app on Linux is started by a shell script.** The runtime mounts
//! the image, says where it mounted it in `APPDIR`, and execs the `AppRun`
//! electron-builder writes — which leads four variables with directories inside
//! that mount before it execs the browser:
//!
//!     PATH="$APPDIR:$APPDIR/usr/sbin:$PATH"
//!     XDG_DATA_DIRS="$APPDIR/usr/share/:…"
//!     LD_LIBRARY_PATH="$APPDIR/usr/lib:…"
//!     GSETTINGS_SCHEMA_DIR="$APPDIR/usr/share/glib-2.0/schemas:…"
//!
//! **Those are Electron's to run over and nobody else's.** The sidecar inherits
//! this process's environment — see [`start`](./sidecar.js) — the server it runs
//! inherits it from the sidecar, and a session's `PATH` is the server's own with
//! Verkstead's directory in front of it. So a `PATH` led by the mount is a mount
//! on every session's `PATH`, pointing at a directory that goes when the app
//! does; and a bundled library reaching a host binary is the oldest way an
//! AppImage breaks something outside itself, with the CLI shelling out to `git`
//! for a Set's project, its branch and its Diff.
//!
//! **Entry by entry rather than variable by variable.** `AppRun` keeps what it
//! was given — each of the four ends in whatever the human had exported — so
//! deleting the variable outright would take their `XDG_DATA_DIRS` away with the
//! bundle's. What comes out is every entry *inside* the mount, and a variable
//! left holding nothing comes out with them.
//!
//! **`APPDIR` itself stays.** It is what says this run came out of a mount, and
//! nothing downstream reads it any more: the server's own reading of it — a
//! launcher in front of the image and a bind of the `usr/lib` beside it, written
//! for the Rust AppImage and wrong for this one — is gone (ADR-0020), the CLI a
//! packed app carries being the static musl build with no loader to point
//! anywhere.
//!
//! Nothing here touches the filesystem or the running process: an environment
//! goes in and the one the child gets comes back, which is what lets the whole
//! of it be an ordinary unit test.

/// What the AppImage runtime says about where it mounted this run.
export const APPDIR = "APPDIR";

/// And the variables its launcher leads with directories inside that mount,
/// each of them a `:`-separated list.
export const LED = [
  "PATH",
  "LD_LIBRARY_PATH",
  "XDG_DATA_DIRS",
  "GSETTINGS_SCHEMA_DIR",
] as const;

/// An environment, which is the shape `process.env` has.
export type Environment = Partial<Record<string, string>>;

/// Whether an entry of one of those lists is inside the mount.
const inside = (entry: string, appdir: string): boolean =>
  entry === appdir || entry.startsWith(`${appdir}/`);

/// The environment to hand a child, out of the one this process was given.
///
/// Every other environment is passed through as it stands. An `APPDIR` that is
/// not an absolute POSIX path is not the runtime's — the same thing the server
/// asked of it before it trusted it for anything — and a checkout run, a Mac and
/// a Windows install have no such variable at all.
export function unmounted(env: Environment): Environment {
  const appdir = env[APPDIR];

  if (appdir === undefined || !appdir.startsWith("/")) {
    return env;
  }

  const passed: Environment = { ...env };

  for (const name of LED) {
    const value = passed[name];
    if (value === undefined) {
      continue;
    }

    const kept = value.split(":").filter((entry) => !inside(entry, appdir));

    if (kept.length === 0) {
      delete passed[name];
    } else {
      passed[name] = kept.join(":");
    }
  }

  return passed;
}

/// Which of them this run took anything out of, for the app's own log.
///
/// Empty for every run that is not a packed Linux one, which is why the line is
/// written only where this says something: a Mac has nothing to report and a
/// line per launch saying so is a line in a file somebody is asked to send.
export function stripped(env: Environment, passed: Environment): string[] {
  return LED.filter((name) => passed[name] !== env[name]);
}
