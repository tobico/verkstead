//! What the menu holds, platform by platform — a value, and every one of its
//! items one of the platform's own roles.
//!
//! The menu itself is Electron's: built from this template and set on the
//! application by [`menu.ts`](./menu.js), which is the file at the wall in
//! `eslint.config.js`. What is here is the half that is a value — which
//! submenus there are, in which order, and which role stands in each — so that
//! vitest can be asked what this platform's menu carries, exactly as
//! [`chosen.ts`](./chosen.js) is the tray menu's answerable half.
//!
//! **Everything is a role rather than a handler.** A role is the platform's own
//! item — its label in the machine's language, its accelerator in the
//! platform's spelling, and its behaviour written by people who know what Cmd+Z
//! means in a text field on a Mac. So there is nothing here to get wrong but
//! which of them a platform gets.
//!
//! **Which is the one thing worth pinning.** On Windows and Linux the bar is
//! hidden (ADR-0020) and the menu is the whole of what registers the keystrokes
//! a window showing a web page is expected to answer, so an item dropped from
//! it is a shortcut that silently stops working. And on a Mac the appMenu and
//! the windowMenu are furniture the platform fills in for itself, which the
//! other two have neither of.
//!
//! **The Window submenu is Minimize, and it is there for the two platforms
//! whose bar is hidden.** Stage 04 of the Electron roadmap measured the
//! controls overlay at one button on a Wayland COSMIC — a close and nothing
//! else, which is Chromium's doing rather than the compositor's — and COSMIC
//! binds no minimise key of its own out of the box, so a human there had no
//! gesture at all for putting the window away. Maximise they have, a
//! double-click on any pane head being one; this is the other. It draws nothing
//! on a hidden bar and carries the platform's own accelerator, which is what
//! makes it the cheapest of the three answers that stage left open — not a
//! control the page draws, which ADR-0020 turned down, and not the X11 path,
//! which would give up Wayland-native rendering on every Wayland desktop to fix
//! one. A Mac has it already, in the windowMenu below and on the same keystroke.

/// The roles this menu is made of, named rather than taken from Electron.
///
/// The list is what a typo fails against: `menu.ts` hands this template to
/// `Menu.buildFromTemplate`, so a role Electron does not have is a typecheck
/// that fails at that one line rather than a menu item that quietly does
/// nothing.
export type Role =
  | "undo"
  | "redo"
  | "cut"
  | "copy"
  | "paste"
  | "selectAll"
  | "reload"
  | "resetZoom"
  | "zoomIn"
  | "zoomOut"
  | "togglefullscreen"
  | "toggleDevTools"
  | "minimize"
  | "appMenu"
  | "windowMenu";

/// One item of the template, in as much of Electron's own shape as this menu
/// uses.
///
/// Electron's `MenuItemConstructorOptions` is what the template is handed over
/// as, and it comes out of `electron` — which this module cannot import and
/// still be a module vitest runs. So the part of that shape this menu is made of
/// is written here, and `menu.ts` is where the two are held against each other.
export interface Item {
  /// The platform's own item, where this is one.
  readonly role?: Role;
  /// A rule between them.
  readonly type?: "separator";
  /// What a submenu is called. The items themselves are labelled by their
  /// roles.
  readonly label?: string;
  /// What is under it.
  readonly submenu?: Item[];
}

/// The menu this platform gets.
export function roles(platform: NodeJS.Platform): Item[] {
  const editing: Item = {
    label: "Edit",
    submenu: [
      { role: "undo" },
      { role: "redo" },
      { type: "separator" },
      { role: "cut" },
      { role: "copy" },
      { role: "paste" },
      { role: "selectAll" },
    ],
  };

  const view: Item = {
    label: "View",
    submenu: [
      { role: "reload" },
      { type: "separator" },
      { role: "resetZoom" },
      { role: "zoomIn" },
      { role: "zoomOut" },
      { type: "separator" },
      { role: "togglefullscreen" },
      // Kept on a release build as well as a development one: what somebody is
      // asked for when the workbench draws wrong is what the console says, and
      // an app that cannot be asked is one whose problems are reproduced by
      // guesswork.
      { role: "toggleDevTools" },
    ],
  };

  // The way to put the window away where the platform's controls do not offer
  // one — see the top of this file. A Mac's is the windowMenu's own.
  const window: Item = { label: "Window", submenu: [{ role: "minimize" }] };

  // The Mac's own furniture: the application menu it cannot do without, and the
  // Window menu that holds Minimize and Zoom — both of which the platform fills
  // in for itself.
  return platform === "darwin"
    ? [{ role: "appMenu" }, editing, view, { role: "windowMenu" }]
    : [editing, view, window];
}
