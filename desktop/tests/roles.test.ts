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

import { ACTS, GUIDE, type Item, ISSUES, roles } from "../src/roles.js";

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

describe("a Mac's bar", () => {
  /// What each submenu holds, as labels, ids and keys — a role's label and key
  /// being the platform's, it is named by its role.
  function drawn(items: Item[]): unknown[] {
    return items.map((item) =>
      item.type === "separator"
        ? "—"
        : item.role !== undefined
          ? item.role
          : { id: item.id, label: item.label, accelerator: item.accelerator },
    );
  }

  it("is the application menu, File, Edit, View, Window and Help", () => {
    const mac = roles("darwin");
    expect(mac.map((item) => item.label ?? item.role)).toStrictEqual([
      "Verkstead",
      "File",
      "Edit",
      "View",
      "windowMenu",
      "Help",
    ]);
  });

  /// Written out rather than the `appMenu` role, so that Settings… stands where
  /// a Mac puts it — every other item still the platform's own.
  it("has Settings… on ⌘, in the application menu", () => {
    expect(drawn(under(roles("darwin"), "Verkstead") ?? [])).toStrictEqual([
      "about",
      "—",
      { id: "settings", label: "Settings…", accelerator: "Cmd+," },
      "—",
      "services",
      "—",
      "hide",
      "hideOthers",
      "unhide",
      "—",
      "quit",
    ]);
    expect(within(roles("darwin"))).not.toContain("appMenu");
  });

  it("has New Conversation on ⌘N and the platform's Close Window in File", () => {
    expect(drawn(under(roles("darwin"), "File") ?? [])).toStrictEqual([
      { id: "compose", label: "New Conversation", accelerator: "Cmd+N" },
      "close",
    ]);
  });

  /// The `help` role is what gives the menu its search field.
  it("has Verkstead Help, View Logs and Report an Issue under the help role", () => {
    const help = roles("darwin").find((item) => item.label === "Help");
    expect(help?.role).toBe("help");
    expect(drawn(help?.submenu ?? [])).toStrictEqual([
      { id: "guide", label: "Verkstead Help", accelerator: undefined },
      { id: "logs", label: "View Logs", accelerator: undefined },
      { id: "issue", label: "Report an Issue", accelerator: undefined },
    ]);
  });

  /// Every act the edge maps is on the bar once, and nothing on the bar is an
  /// act the edge has not got.
  it("carries every act once", () => {
    const ids = (items: Item[]): string[] =>
      items.flatMap((item) => [
        ...(item.id === undefined ? [] : [item.id]),
        ...ids(item.submenu ?? []),
      ]);
    expect(ids(roles("darwin")).sort()).toStrictEqual([...ACTS].sort());
  });

  it("goes to the docs site and the issue tracker", () => {
    expect(GUIDE).toBe("https://tobico.github.io/verkstead/");
    expect(ISSUES).toBe("https://github.com/tobico/verkstead/issues");
  });

  /// The Window menu the platform fills in for itself, and none of it anywhere
  /// near the other two platforms, which have no strip at the top of the
  /// screen to draw it in.
  it("is a Mac's alone", () => {
    const mac = roles("darwin");
    expect(mac).toContainEqual({ role: "windowMenu" });

    for (const platform of HIDDEN) {
      const theirs = roles(platform);
      expect(theirs.map((item) => item.label), platform).toStrictEqual(["Edit", "View", "Window"]);
      expect(JSON.stringify(theirs), platform).not.toMatch(/"(id|accelerator)"/);
    }
  });
});
