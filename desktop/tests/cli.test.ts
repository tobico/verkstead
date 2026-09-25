//! Which `verkstead` the app starts — every arm of it, none of which needs a
//! filesystem or an Electron.
//!
//! Worth a test of its own because it is three answers that look like one, and
//! each of them is a way of being quietly wrong: a checkout that reaches past
//! the repository root, a packed app that names the install root and so answers
//! a `PATH` lookup with the window, an override that is read but not resolved.

import { basename, resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { cli, OVERRIDE, type Install } from "../src/cli.js";

/// A checkout: the main process loaded out of `desktop/dist`, which is where
/// `tsconfig.build.json` puts it.
const CHECKOUT: Install = {
  packaged: false,
  entry: "/home/dev/verkstead/desktop/dist",
  resources: "/home/dev/verkstead/desktop/dist/resources",
  platform: "linux",
  env: {},
};

/// And a packed app, whose resources are somewhere inside the install.
const PACKED: Install = {
  packaged: true,
  entry: "/opt/Verkstead/resources/app/dist",
  resources: "/opt/Verkstead/resources",
  platform: "linux",
  env: {},
};

describe("where the sidecar's binary is", () => {
  it("is the workspace's own build in a checkout", () => {
    expect(cli(CHECKOUT)).toBe("/home/dev/verkstead/target/debug/verkstead");
  });

  it("is a directory of its own inside a packed app, never the install root", () => {
    expect(cli(PACKED)).toBe("/opt/Verkstead/resources/cli/verkstead");
  });

  /// The name is the platform's rather than the install's — a Windows
  /// developer's checkout is as much a checkout. The name alone is asserted:
  /// the separators around it are the *host's*, which is what they have to be,
  /// this being a path that host is about to spawn.
  it("carries the platform's own file name", () => {
    expect(basename(cli({ ...CHECKOUT, platform: "win32" }))).toBe("verkstead.exe");
    expect(basename(cli({ ...PACKED, platform: "darwin" }))).toBe("verkstead");
  });

  it("is what the environment says, over either of them", () => {
    const said = { [OVERRIDE]: "/usr/local/bin/verkstead" };
    expect(cli({ ...CHECKOUT, env: said })).toBe("/usr/local/bin/verkstead");
    expect(cli({ ...PACKED, env: said })).toBe("/usr/local/bin/verkstead");
  });

  /// A variable is typed into a shell, and a shell's notion of a relative path
  /// is the directory the app was started from.
  it("resolves a relative override against where the app was started", () => {
    const said = { [OVERRIDE]: "target/release/verkstead" };
    expect(cli({ ...CHECKOUT, env: said })).toBe(resolve("target/release/verkstead"));
  });

  /// An empty variable is one a shell exported without a value —
  /// `VERKSTEAD_CLI=` in an environment file, say — and it says nothing rather
  /// than naming the binary at the empty path.
  it("ignores an override with nothing in it", () => {
    expect(cli({ ...CHECKOUT, env: { [OVERRIDE]: "" } })).toBe(
      "/home/dev/verkstead/target/debug/verkstead",
    );
  });
});
