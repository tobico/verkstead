//! What the sidecar is handed, out of the environment a packed app was given.
//!
//! The four variables below are the real ones, written as electron-builder's
//! `AppRun` writes them — read off the `AppRun` inside a
//! `Verkstead-x86_64.AppImage` this branch packed rather than out of its
//! documentation, because the documentation says nothing about the two the
//! script appends defaults to.

import { describe, expect, it } from "vitest";

import { APPDIR, LED, stripped, unmounted } from "../src/unmounted.js";

/// Where the runtime mounts an image: `/tmp/.mount_` and six random characters.
const MOUNT = "/tmp/.mount_VerksdVCPsN";

/// The environment a packed run is given, `AppRun` having led all four with the
/// mount — and, for each, whatever the human had exported behind it.
///
/// `XDG_DATA_DIRS` is the odd one: the script appends three fixed system
/// directories of its own after what it was given, so the mount's entry is one
/// of five rather than the whole of it.
const packed = (theirs: Partial<Record<string, string>> = {}) => ({
  [APPDIR]: MOUNT,
  APPIMAGE: "/home/someone/Applications/Verkstead-x86_64.AppImage",
  HOME: "/home/someone",
  PATH: `${MOUNT}:${MOUNT}/usr/sbin:/run/current-system/sw/bin:/usr/bin`,
  XDG_DATA_DIRS: `${MOUNT}/usr/share/:/usr/share/gnome:/usr/local/share/:/usr/share/`,
  LD_LIBRARY_PATH: `${MOUNT}/usr/lib`,
  GSETTINGS_SCHEMA_DIR: `${MOUNT}/usr/share/glib-2.0/schemas`,
  ...theirs,
});

describe("the environment a sidecar is handed", () => {
  /// The loader and the schemas held nothing but the mount, so they go
  /// altogether: an empty `LD_LIBRARY_PATH` is not the same as no
  /// `LD_LIBRARY_PATH` — the loader reads one as the current directory.
  it("has nothing of the mount left in it", () => {
    const passed = unmounted(packed());

    expect(passed.LD_LIBRARY_PATH).toBeUndefined();
    expect(passed.GSETTINGS_SCHEMA_DIR).toBeUndefined();
    expect(passed.PATH).toBe("/run/current-system/sw/bin:/usr/bin");
    expect(passed.XDG_DATA_DIRS).toBe("/usr/share/gnome:/usr/local/share/:/usr/share/");

    // None of the four, whatever else the environment holds: `APPDIR` and
    // `APPIMAGE` are the runtime's own and name the mount on purpose, which the
    // third test below is about.
    for (const name of LED) {
      expect(passed[name] ?? "", `${name} names the mount`).not.toContain(MOUNT);
    }
  });

  /// Which is why it is done entry by entry: the human's own `LD_LIBRARY_PATH`
  /// and `XDG_DATA_DIRS` are behind the bundle's in the same variable, and
  /// deleting the variable would take a desktop's data directories away from
  /// every session under this server.
  it("keeps what the human exported behind it", () => {
    const passed = unmounted(
      packed({
        LD_LIBRARY_PATH: `${MOUNT}/usr/lib:/opt/theirs/lib`,
        GSETTINGS_SCHEMA_DIR: `${MOUNT}/usr/share/glib-2.0/schemas:/opt/theirs/schemas`,
        XDG_DATA_DIRS: `${MOUNT}/usr/share/:/home/someone/.local/share:/usr/share/`,
      }),
    );

    expect(passed.LD_LIBRARY_PATH).toBe("/opt/theirs/lib");
    expect(passed.GSETTINGS_SCHEMA_DIR).toBe("/opt/theirs/schemas");
    expect(passed.XDG_DATA_DIRS).toBe("/home/someone/.local/share:/usr/share/");
  });

  /// And nothing else is touched, `APPDIR` included: it is what says this run
  /// came out of a mount, and the one thing that ever read it downstream — the
  /// server's launcher in front of its own image — is gone.
  it("leaves the rest of the environment alone", () => {
    const given = packed({ VERKSTEAD_DATA_DIR: "/srv/verkstead" });
    const passed = unmounted(given);

    expect(passed[APPDIR]).toBe(MOUNT);
    expect(passed.APPIMAGE).toBe(given.APPIMAGE);
    expect(passed.HOME).toBe("/home/someone");
    expect(passed.VERKSTEAD_DATA_DIR).toBe("/srv/verkstead");
  });

  /// A checkout run, a Mac and a Windows install have no `APPDIR` at all, and a
  /// developer's own `LD_LIBRARY_PATH` is theirs — the dev shell exports one.
  it("passes an environment that came out of no mount straight through", () => {
    const given = {
      HOME: "/home/someone",
      PATH: "/run/current-system/sw/bin",
      LD_LIBRARY_PATH: "/nix/store/whatever/lib",
    };

    expect(unmounted(given)).toBe(given);
    expect(stripped(given, unmounted(given))).toEqual([]);
  });

  /// And an `APPDIR` that is not an absolute path is not the runtime's, which is
  /// what the server asked of the same variable before it trusted it: a relative
  /// one filtered as a prefix would take entries out for matching a few
  /// characters.
  it("trusts nothing but an absolute mount", () => {
    const given = { [APPDIR]: "usr", PATH: "/usr/bin:/usr/sbin" };

    expect(unmounted(given).PATH).toBe("/usr/bin:/usr/sbin");
  });

  /// What the app's log line names, which is the variables that changed and not
  /// the ones that were only read.
  it("says which of them it took something out of", () => {
    const given = packed();

    expect(stripped(given, unmounted(given))).toEqual([...LED]);

    const loader = { [APPDIR]: MOUNT, LD_LIBRARY_PATH: `${MOUNT}/usr/lib`, HOME: "/home/x" };
    expect(stripped(loader, unmounted(loader))).toEqual(["LD_LIBRARY_PATH"]);
  });
});
