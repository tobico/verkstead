//! What a press of the close button comes to, out of the two settings and the
//! platform.
//!
//! Three decisions live here and each of them is one somebody could get wrong
//! in a way nobody notices until the app is on a desk: the fall-through that
//! keeps a Verkstead with no icon from hiding its only window, the Mac arm that
//! never consults the radio at all, and the warning being the close button's
//! alone.

import { describe, expect, it } from "vitest";

import { CANCEL, closing, QUIT, WARNING } from "../src/closing.js";
import { DEFAULTS, POSITIONS, type Settings } from "../src/settings.js";

/// A desk with an icon on its panel, which is every machine that has not turned
/// one off.
const withTray = (whenClosed: Settings["whenClosed"]): Settings => ({
  whenClosed,
  trayIcon: true,
});

/// And the same desk with the icon turned off.
const without = (whenClosed: Settings["whenClosed"]): Settings => ({
  whenClosed,
  trayIcon: false,
});

describe("the three positions", () => {
  it("keeps running in the tray", () => {
    expect(closing(withTray("tray"), "linux")).toBe("hide");
  });

  it("asks before quitting", () => {
    expect(closing(withTray("ask"), "linux")).toBe("ask");
  });

  it("quits", () => {
    expect(closing(withTray("quit"), "linux")).toBe("quit");
  });

  /// The app somebody who has never opened the Desktop page is using.
  it("keeps running in the tray by default", () => {
    expect(closing(DEFAULTS, "linux")).toBe("hide");
    expect(closing(DEFAULTS, "win32")).toBe("hide");
  });

  /// Linux and Windows read the radio the same way: the Dock is the one
  /// platform difference, and it is the arm below.
  it("reads the same on Windows as on Linux", () => {
    for (const whenClosed of POSITIONS) {
      expect(closing(withTray(whenClosed), "win32"), whenClosed).toBe(
        closing(withTray(whenClosed), "linux"),
      );
    }
  });
});

describe("with the tray off", () => {
  /// Because a hidden app with no icon is a Verkstead nobody can reach, and
  /// there would be no way back to it but the command line.
  it("quits whatever the radio says", () => {
    for (const whenClosed of POSITIONS) {
      expect(closing(without(whenClosed), "linux"), whenClosed).toBe("quit");
      expect(closing(without(whenClosed), "win32"), whenClosed).toBe("quit");
    }
  });

  /// And not *ask first*, which was the other way the fall-through could have
  /// gone: a dialog that offers to hide a window nobody can bring back is a
  /// dialog offering to lose the app.
  it("does not ask first", () => {
    expect(closing(without("ask"), "linux")).not.toBe("ask");
  });

  /// The setting is not overwritten by any of this — it is read past, and
  /// turning the icon back on is the position doing what it says again.
  it("restores the position when the icon comes back", () => {
    const chosen: Settings = { whenClosed: "tray", trayIcon: false };

    expect(closing(chosen, "linux")).toBe("quit");
    expect(closing({ ...chosen, trayIcon: true }, "linux")).toBe("hide");
  });
});

describe("a Mac", () => {
  /// Closing a window on a Mac leaves the application running in the Dock, and
  /// a Dock activation brings it back. The page draws no radio there, so a
  /// settings file carried over from another machine has nothing to say.
  it("leaves the app running whatever the radio says", () => {
    for (const whenClosed of POSITIONS) {
      expect(closing(withTray(whenClosed), "darwin"), whenClosed).toBe("hide");
    }
  });

  /// Including with the menu bar icon off: the Dock is the way back there, and
  /// it is not a thing the app can turn off.
  it("leaves the app running with the icon off too", () => {
    for (const whenClosed of POSITIONS) {
      expect(closing(without(whenClosed), "darwin"), whenClosed).toBe("hide");
    }
  });
});

describe("the warning", () => {
  /// What it is for: a Verkstead that quits takes its sessions with it.
  it("says what quitting costs", () => {
    expect(WARNING.detail).toContain("sessions");
    expect(WARNING.buttons).toEqual(["Quit", "Cancel"]);
  });

  /// The one that changes nothing is the one the window's own close button and
  /// the Escape key come to.
  it("has Quit and Cancel where it says they are", () => {
    expect(WARNING.buttons[QUIT]).toBe("Quit");
    expect(WARNING.buttons[CANCEL]).toBe("Cancel");
  });

  /// And nothing but the ask reaches it: the tray's Quit and the menu's Quit
  /// are not closes, and the positions that are not "ask" never put one up.
  it("is asked for by one position only", () => {
    const asks = POSITIONS.filter((whenClosed) => closing(withTray(whenClosed), "linux") === "ask");

    expect(asks).toEqual(["ask"]);
  });
});
