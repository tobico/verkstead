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

import {
  ASKED,
  HEAD,
  INSTALLS,
  LOGS,
  NAME,
  NOTIFIABLE,
  OPEN,
  PLACED,
  PRELOAD,
  REGISTER,
  SET,
  STARTUP,
  WHERE,
} from "../src/bridge.js";

/// Every channel the bridge crosses on, which is the list both tests below read.
/// [`HEAD`] and [`WHERE`] among them though they go the other way: a channel is
/// a name in a renderer's namespace whichever direction the message travels in,
/// and a push that shared a name with a question would be an overlay recoloured by
/// a set.
const CHANNELS = [ASKED, SET, LOGS, STARTUP, REGISTER, NOTIFIABLE, PLACED, INSTALLS, HEAD, WHERE, OPEN];

/// This project's own directory, which is what `src/` is under.
const project = dirname(dirname(fileURLToPath(import.meta.url)));

describe("the channels", () => {
  it("names each act once", () => {
    expect(new Set(CHANNELS).size).toBe(CHANNELS.length);
  });

  /// A channel is a name in a renderer's whole namespace of them, so the app's
  /// own say whose they are.
  it("carries the app's name", () => {
    for (const channel of CHANNELS) {
      expect(channel, channel).toMatch(/^verkstead:/);
    }
  });
});

/// The one channel the page's side of the bridge names as well, because the page
/// listens on it through the preload — so it is pinned here and in
/// `web/tests/opened.test.ts` to the same literal.
describe("the channel a clicked notification is told on", () => {
  it("is the app's own word for opening", () => {
    expect(OPEN).toBe("verkstead:open");
  });
});

/// And the one the page pushes the Go menu's list on, which it names for the same
/// reason — pinned here and in `web/tests/where.test.tsx` to the same literal.
describe("the channel what the open Conversation can go to is said on", () => {
  it("is the app's own word for where", () => {
    expect(WHERE).toBe("verkstead:where");
  });
});

/// And the two the Desktop page's Install row is drawn from and pressed on,
/// which the page's side says again — pinned here and in
/// `web/tests/desktop.test.tsx` to the same literals.
describe("the channels the Install row asks on", () => {
  it("are the app's own words for where it runs from and installing", () => {
    expect(PLACED).toBe("verkstead:placed");
    expect(INSTALLS).toBe("verkstead:install");
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
