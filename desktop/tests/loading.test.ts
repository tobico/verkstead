//! The wording of a failed load, and the secret it must never carry.
//!
//! The window is loaded on the login link, and a rejected `loadURL` is an
//! Electron error whose message is the description, the number **and the URL it
//! was given** — so the test that matters here is the one that puts a real key
//! in and asks whether it comes out. The errors below are shaped as Electron
//! shapes them, message and fields both, taken from a measured rejection:
//!
//! ```
//! Error: ERR_UNSAFE_PORT (-312) loading 'http://127.0.0.1:9/?key=aRealLookingSecret'
//! ```

import { describe, expect, it } from "vitest";

import { why } from "../src/loading.js";

/// A rejection as Electron makes one: the message it composes, and the two
/// fields it hangs on the error beside it.
function refused(code: string, errno: number, url: string): Error {
  return Object.assign(new Error(`${code} (${errno}) loading '${url}'`), {
    code,
    errno,
    url,
  });
}

/// The link the window is actually loaded on, with a key the test can look for.
const SECRET = "aRealLookingSecret";
const LINK = `http://127.0.0.1:8422/?key=${SECRET}`;

describe("why a load did not happen", () => {
  it("is the description and the number behind it", () => {
    expect(why(refused("ERR_CONNECTION_REFUSED", -102, LINK))).toBe(
      "ERR_CONNECTION_REFUSED (-102)",
    );
  });

  /// The whole reason this is a function with a test rather than a
  /// `String(trouble)` at the call site: the window is loaded on the login
  /// link, the log file is what **View Logs** opens on somebody's desk, and
  /// Electron's own message carries the URL it was handed (ADR-0015).
  it("never carries the workbench key, whatever the error was holding", () => {
    for (const trouble of [
      refused("ERR_CONNECTION_REFUSED", -102, LINK),
      refused("ERR_ABORTED", -3, LINK),
      refused("ERR_UNSAFE_PORT", -312, LINK),
      new Error(`ERR_FAILED (-2) loading '${LINK}'`),
      LINK,
    ]) {
      const said = why(trouble);

      expect(said, said).not.toContain(SECRET);
      expect(said, said).not.toContain("key=");
      expect(said, said).not.toContain("127.0.0.1");
    }
  });

  /// An error carrying only the description is still worth the description.
  it("is the description alone where there is no number", () => {
    expect(why(Object.assign(new Error("boom"), { code: "ERR_FAILED" }))).toBe("ERR_FAILED");
  });

  /// And an error whose whole account of itself is its message is not printed
  /// at all, because that message is the thing being kept out of the file.
  it("says so rather than repeating a failure it cannot read", () => {
    for (const trouble of [
      new Error(`ERR_FAILED (-2) loading '${LINK}'`),
      LINK,
      undefined,
      null,
      42,
      {},
      Object.assign(new Error("boom"), { code: "" }),
      Object.assign(new Error("boom"), { code: 7 }),
    ]) {
      expect(why(trouble), String(trouble)).toBe(
        "the reason was nothing this can report without repeating it",
      );
    }
  });
});
