//! Where the **Data Directory** is, on all three platforms at once.
//!
//! The app has to land on the directory the sidecar landed on — the key it
//! reads is the file that server wrote — so this is the server's own
//! `crates/server/src/platform.rs` asked the same questions, arm for arm. Every
//! arm runs on the Linux runner, which is the whole reason the platform and the
//! environment are values rather than reads of this process.

import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { dataDir, SAID, type Machine } from "../src/platform.js";

/// A Unix machine that has never heard of XDG: a home and nothing else.
const withHome = (platform: NodeJS.Platform, home: string): Machine => ({
  platform,
  env: { HOME: home },
});

describe("where the Data Directory is when nobody has said", () => {
  it("is under the XDG data directory on Linux", () => {
    expect(dataDir(withHome("linux", "/home/you"))).toBe("/home/you/.local/share/verkstead");
  });

  it("honours an absolute XDG_DATA_HOME", () => {
    const machine = withHome("linux", "/home/you");
    machine.env.XDG_DATA_HOME = "/var/lib/data";

    expect(dataDir(machine)).toBe("/var/lib/data/verkstead");
  });

  /// A relative value would resolve against whatever directory the app happened
  /// to be started in, which is what the platform default is here to stop.
  it("ignores a relative XDG_DATA_HOME", () => {
    const machine = withHome("linux", "/home/you");
    machine.env.XDG_DATA_HOME = "data";

    expect(dataDir(machine)).toBe("/home/you/.local/share/verkstead");
  });

  it("is in Application Support on macOS", () => {
    expect(dataDir(withHome("darwin", "/Users/you"))).toBe(
      "/Users/you/Library/Application Support/Verkstead",
    );
  });

  it("reads no XDG variable on macOS, whoever exported it", () => {
    const machine = withHome("darwin", "/Users/you");
    machine.env.XDG_DATA_HOME = "/var/lib/data";

    expect(dataDir(machine)).toBe("/Users/you/Library/Application Support/Verkstead");
  });

  /// The roaming application data, because everything in this directory is
  /// meant to follow the human between machines. The separators are the host's
  /// own, as they are in `cli.ts`: this is a path that host is about to read.
  it("is in the roaming application data on Windows", () => {
    const machine: Machine = {
      platform: "win32",
      env: { APPDATA: "C:\\Users\\you\\AppData\\Roaming" },
    };

    expect(dataDir(machine)).toBe(join("C:\\Users\\you\\AppData\\Roaming", "Verkstead"));
  });

  /// A Unix home on a Windows machine was set by somebody's shell rather than
  /// by the platform, and says nothing about where its application data goes.
  it("reads no HOME on Windows", () => {
    expect(dataDir(withHome("win32", "/home/you"))).toBeUndefined();
  });

  it("is nowhere on every platform with nothing in the environment", () => {
    for (const platform of ["linux", "darwin", "win32"] as const) {
      expect(dataDir({ platform, env: {} }), `${platform} has nowhere to resolve to`).toBeUndefined();
    }
  });

  /// A platform the server resolves as Linux resolves as Linux here too — the
  /// server's own `Platform::HERE` maps everything that is neither a Mac nor
  /// Windows to the XDG arm.
  it("takes a platform that is neither a Mac nor Windows for the XDG one", () => {
    expect(dataDir(withHome("freebsd", "/home/you"))).toBe("/home/you/.local/share/verkstead");
  });
});

describe(`the Data Directory ${SAID} says`, () => {
  it("is what it says, over every platform's own place for one", () => {
    for (const platform of ["linux", "darwin", "win32"] as const) {
      expect(dataDir({ platform, env: { [SAID]: "/srv/somewhere", HOME: "/home/you" } })).toBe(
        "/srv/somewhere",
      );
    }
  });

  /// Taken as it was said, unresolved: the sidecar is started with this
  /// process's own environment and its own working directory, so a relative
  /// value is the same directory for both of them — which is what
  /// `--data-dir .` out of a checkout has always meant.
  it("is taken as it was said, relative and all", () => {
    expect(dataDir({ platform: "linux", env: { [SAID]: ".", HOME: "/home/you" } })).toBe(".");
  });

  /// `VERKSTEAD_DATA_DIR=` in an environment file is a variable exported
  /// without a value, and it says nothing rather than naming the empty path.
  it("says nothing when it is empty", () => {
    expect(dataDir({ platform: "linux", env: { [SAID]: "", HOME: "/home/you" } })).toBe(
      "/home/you/.local/share/verkstead",
    );
  });
});
