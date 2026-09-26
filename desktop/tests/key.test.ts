//! The **Workbench Key** as the app reads it, and the link it builds out of one.
//!
//! Both halves are worth a test because both are ways of being quietly wrong:
//! a read that kept the newline the server writes would build a link with a
//! secret nothing matches, and a link that escaped the secret would meet a gate
//! that splits its query by hand rather than decoding it.

import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { KEY_FILE, keyIn, link } from "../src/key.js";
import { ORIGIN } from "../src/workbench.js";

/// A Data Directory of its own, with whatever this test wants in it.
let data: string | undefined;

function dataDirectory(): string {
  data = mkdtempSync(join(tmpdir(), "verkstead-key-"));
  return data;
}

afterEach(() => {
  if (data !== undefined) {
    rmSync(data, { recursive: true, force: true });
    data = undefined;
  }
});

describe("the key in the Data Directory", () => {
  /// The server writes the secret with a newline after it — see
  /// `WorkbenchKey::issued` — and a key with that newline still on it is a link
  /// the gate refuses.
  it("is the file's contents with the newline the server wrote taken off", () => {
    const where = dataDirectory();
    writeFileSync(join(where, KEY_FILE), "aRealLookingSecret\n");

    expect(keyIn(where)).toBe("aRealLookingSecret");
  });

  it("is nothing where there is no file yet", () => {
    expect(keyIn(dataDirectory())).toBeUndefined();
  });

  /// A file that is there and empty is a file that is not there, which is what
  /// the server makes of one as well: nothing writes an empty one, so it is a
  /// machine that lost power or a hand that emptied it.
  it("is nothing where the file is empty", () => {
    const where = dataDirectory();
    writeFileSync(join(where, KEY_FILE), "\n");

    expect(keyIn(where)).toBeUndefined();
  });

  it("is nothing where the directory is not there at all", () => {
    expect(keyIn(join(dataDirectory(), "not-made-yet"))).toBeUndefined();
  });

  /// Read afresh rather than remembered: **Reset key** on the phone writes this
  /// file, and a link built from what was read at startup is a 401 with extra
  /// steps.
  it("is read again every time it is asked for", () => {
    const where = dataDirectory();
    writeFileSync(join(where, KEY_FILE), "theFirstOne\n");
    expect(keyIn(where)).toBe("theFirstOne");

    writeFileSync(join(where, KEY_FILE), "theOneAfterTheReset\n");
    expect(keyIn(where)).toBe("theOneAfterTheReset");
  });
});

describe("the login link", () => {
  it("is the address with the key on it, at the root", () => {
    expect(link("http://127.0.0.1:8422", "aSecret")).toBe("http://127.0.0.1:8422/?key=aSecret");
  });

  /// The workbench's own origin has no trailing slash, and an origin that grew
  /// one is still one path rather than two.
  it("leaves one slash whatever the origin ended with", () => {
    expect(link(`${ORIGIN}/`, "aSecret")).toBe(link(ORIGIN, "aSecret"));
  });

  /// The gate reads the parameter by splitting the query rather than decoding
  /// it — see `key::offered_in` — so a link that escaped the secret would be a
  /// secret that never matched. The alphabet a key is spelled in has nothing a
  /// URL escapes, which is what makes that safe at both ends.
  it("hands the secret over exactly as the file spells it", () => {
    expect(link(ORIGIN, "-_abcXYZ0189")).toBe(`${ORIGIN}/?key=-_abcXYZ0189`);
  });
});
