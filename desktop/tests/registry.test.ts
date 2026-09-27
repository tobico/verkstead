//! `reg.exe` as this app asks it, which is the one reach into a platform
//! **Launch on Startup** makes without an API.
//!
//! **The reading this is here to pin is the one every arm of `startup.ts` gives
//! an unreadable registration**: a value that is not there, a key that is not
//! there and a command that would not run are one answer, which is *nothing is
//! registered*. So this asks about a key no machine has — on Windows, where
//! `reg.exe` runs and says so; on this Linux runner, where there is no `reg.exe`
//! to run at all — and both are the same answer, which is what makes it an
//! ordinary test wherever the suite runs.
//!
//! What is deliberately not here is a value being written: nothing in this app
//! writes one, the registration being Electron's to make, and a suite that wrote
//! under somebody's own Run key would be registering a Verkstead nobody asked
//! for. The two calls this exercises are a read of a key that is not there and a
//! delete of a value that is not there.

import { describe, expect, it } from "vitest";

import { reg, RUN } from "../src/registry.js";

/// A key of this test's own under this user's, named for what it is about so that
/// nothing here ever touches the real one — `run_key.rs`'s own habit, which had
/// the same reason.
const NOWHERE = String.raw`HKCU\Software\net.tobico.Verkstead\tests\nothing`;

describe("where the registration is kept", () => {
  /// The current user's own key, which is the one the tray app wrote and the one
  /// Electron's API writes.
  it("is the current user's Run key", () => {
    expect(RUN).toBe(String.raw`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`);
  });
});

describe("what reg.exe answers", () => {
  it("finds nothing under a key that is not there", () => {
    expect(reg.has(NOWHERE, "Verkstead")).toBe(false);
  });

  /// And a delete that could not be made says so rather than answering, because
  /// the caller has something to do about it: the take-over leaves the value for
  /// the next launch to read again — see `runkey.ts`.
  it("throws where the value could not be deleted", () => {
    expect(() => reg.remove(NOWHERE, "Verkstead")).toThrow();
  });
});
