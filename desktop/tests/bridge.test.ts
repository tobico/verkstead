//! The bridge as a value: the channels it crosses on, and the file the window
//! names as its preload.
//!
//! A module of strings has two ways of going wrong, and they are the two asked
//! about here. Two acts sharing a channel is a set that opens the log file, and
//! nothing about it would look wrong on either side. And a preload the window
//! names but the build does not emit is an app with no bridge at all — Electron
//! says so in a console nobody is reading, the page finds no
//! `window.verkstead`, and what that looks like is the Desktop page quietly not
//! being drawn, which is also exactly what a browser is supposed to see.

import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { ASKED, LOGS, NAME, PRELOAD, SET } from "../src/bridge.js";

/// This project's own directory, which is what `src/` is under.
const project = dirname(dirname(fileURLToPath(import.meta.url)));

describe("the channels", () => {
  it("names each act once", () => {
    expect(new Set([ASKED, SET, LOGS]).size).toBe(3);
  });

  /// A channel is a name in a renderer's whole namespace of them, so the app's
  /// own say whose they are.
  it("carries the app's name", () => {
    for (const channel of [ASKED, SET, LOGS]) {
      expect(channel, channel).toMatch(/^verkstead:/);
    }
  });
});

describe("the preload", () => {
  /// The build constraint, and the reason the source beside it is a `.mts`: a
  /// preload ignores the package's `"type": "module"` and reads the extension
  /// instead, so an ESM preload is an `.mjs` or it is not loaded at all.
  it("is an .mjs, which is the only thing an ESM preload can be", () => {
    expect(PRELOAD.endsWith(".mjs")).toBe(true);
  });

  /// Which `tsc` emits from the `.mts` of the same name and from nothing else.
  /// The one thing this project can check about a compiled-rather-than-bundled
  /// preload without building it: that what the window names has a source to
  /// come from.
  it("is what the build has to emit", () => {
    const source = PRELOAD.replace(/\.mjs$/, ".mts");

    expect(existsSync(join(project, "src", source)), source).toBe(true);
  });
});

describe("the name on the window", () => {
  it("is the product's own", () => {
    expect(NAME).toBe("verkstead");
  });
});
