//! What each platform's menu holds — which is the whole of what a menu of
//! nothing but roles can get wrong.
//!
//! Two things are worth a test here. The **minimise** on Windows and Linux is
//! the only gesture this app has for putting the window away where the
//! platform's own controls overlay draws no button for it, so an item dropped
//! from a bar nobody can see is a window a COSMIC user cannot put down. And the
//! **keystrokes a hidden bar still answers** are registered by this menu and by
//! nothing else, so the same silence is what a lost Edit submenu sounds like.

import { describe, expect, it } from "vitest";

import { type Item, roles } from "../src/roles.js";

/// The two platforms whose menu bar is hidden and whose menu is therefore the
/// whole of what registers a shortcut.
const HIDDEN: NodeJS.Platform[] = ["linux", "win32"];

/// Every role in a template, submenus and all, in the order they are drawn.
function within(items: Item[]): string[] {
  return items.flatMap((item) => [
    ...(item.role === undefined ? [] : [item.role]),
    ...within(item.submenu ?? []),
  ]);
}

/// The submenu drawn under `label`, where there is one.
function under(items: Item[], label: string): Item[] | undefined {
  return items.find((item) => item.label === label)?.submenu;
}

describe("the way to put the window away", () => {
  it("is a Window submenu on the platforms whose bar is hidden", () => {
    for (const platform of HIDDEN) {
      expect(under(roles(platform), "Window"), platform).toStrictEqual([{ role: "minimize" }]);
    }
  });

  /// A Mac's own, in the menu the platform fills in for itself — so an item of
  /// ours beside it would be a second Minimize on the same keystroke.
  it("is the platform's on a Mac", () => {
    const mac = roles("darwin");
    expect(under(mac, "Window")).toBeUndefined();
    expect(within(mac)).toContain("windowMenu");
    expect(within(mac)).not.toContain("minimize");
  });
});

describe("the keystrokes a hidden bar still answers", () => {
  it("are all there on the platforms that hide it", () => {
    for (const platform of HIDDEN) {
      expect(within(roles(platform)), platform).toStrictEqual([
        "undo",
        "redo",
        "cut",
        "copy",
        "paste",
        "selectAll",
        "reload",
        "resetZoom",
        "zoomIn",
        "zoomOut",
        "togglefullscreen",
        "toggleDevTools",
        "minimize",
      ]);
    }
  });

  /// Windows and Linux get the same menu: the Mac arm is the one difference,
  /// and it is the two tests above.
  it("read the same on Windows as on Linux", () => {
    expect(roles("win32")).toStrictEqual(roles("linux"));
  });
});

describe("a Mac's furniture", () => {
  /// The application menu it cannot do without, first, and the Window menu
  /// last — and neither of them anywhere near the other two platforms, which
  /// have no strip at the top of the screen to draw them in.
  it("is the appMenu and the windowMenu, and it is a Mac's alone", () => {
    const mac = roles("darwin");
    expect(mac[0]).toStrictEqual({ role: "appMenu" });
    expect(mac[mac.length - 1]).toStrictEqual({ role: "windowMenu" });

    for (const platform of HIDDEN) {
      expect(within(roles(platform)), platform).not.toContain("appMenu");
    }
  });
});
