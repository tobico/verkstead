//! Where the tray's icon is, and that the file it names is the artwork the tray
//! was promised.
//!
//! Two arms and only one of them exists yet — a checkout, which is every run in
//! this stage, and a packed app, which is stage 05's. Both are a function of
//! where the app is running from, so both are an ordinary unit test; and the
//! one that exists is asked about the file as well as about the path, the way
//! `crates/desktop/src/tray.rs` asked it of the artwork it had built in.

import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { artwork, ARTWORK } from "../src/artwork.js";
import type { Install } from "../src/cli.js";

/// A checkout, run the way `pnpm start` runs it: the main process loaded out of
/// `desktop/dist`, which is two levels under the repository root.
const CHECKOUT: Install = {
  packaged: false,
  entry: "/home/you/verkstead/desktop/dist",
  resources: "/unread",
  platform: "linux",
  env: {},
};

/// The repository this test is in, which is where the real artwork is.
const REPOSITORY = resolve(import.meta.dirname, "..", "..");

/// What the first bytes of a PNG say about it: the signature that says it is
/// one, and the width and height out of the header chunk that follows.
const png = (file: string): { signature: string; width: number; height: number } => {
  const bytes = readFileSync(file);

  return {
    signature: bytes.subarray(1, 4).toString("ascii"),
    width: bytes.readUInt32BE(16),
    height: bytes.readUInt32BE(20),
  };
};

describe("in a checkout", () => {
  it("is the repository's own generated icon", () => {
    expect(artwork(CHECKOUT)).toBe("/home/you/verkstead/assets/icons/icon-192.png");
  });

  /// The one that says the path leads somewhere: a renamed or re-generated file
  /// is a build that still compiles and an app with no icon, which is exactly
  /// what the Rust app's own artwork test was for.
  it("names a file that is there", () => {
    const file = artwork({ ...CHECKOUT, entry: join(REPOSITORY, "desktop", "dist") });

    expect(png(file).signature).toBe("PNG");
  });

  /// And the size the panel was promised: 192 square, which is more than any
  /// panel asks for, because a panel scales down rather than up.
  it("names the 192px one of the set", () => {
    const file = artwork({ ...CHECKOUT, entry: join(REPOSITORY, "desktop", "dist") });

    expect(png(file)).toMatchObject({ width: 192, height: 192 });
  });
});

describe("in a packed app", () => {
  it("is beside the code it was packed with", () => {
    const packed: Install = { ...CHECKOUT, packaged: true, resources: "/opt/Verkstead/resources" };

    expect(artwork(packed)).toBe(join("/opt/Verkstead/resources", ARTWORK));
  });
});
