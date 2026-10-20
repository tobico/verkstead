//! What the menu holds, platform by platform — a value, and every one of its
//! items one of the platform's own roles or one of Verkstead's own acts.
//!
//! The menu itself is Electron's: built from this template and set on the
//! application by [`menu.ts`](./menu.js), which is the file at the wall in
//! `eslint.config.js`. What is here is the half that is a value — which
//! submenus there are, in which order, and which role stands in each — so that
//! vitest can be asked what this platform's menu carries, exactly as
//! [`chosen.ts`](./chosen.js) is the tray menu's answerable half.
//!
//! **Everything is a role or an id, and never a handler.** A role is the
//! platform's own item — its label in the machine's language, its accelerator
//! in the platform's spelling, and its behaviour written by people who know
//! what Cmd+Z means in a text field on a Mac. An id is one of the few items a
//! Mac's bar carries that go somewhere in Verkstead — Settings…, New
//! Conversation, the Help menu's three — and it is mapped to its act at the
//! edge, in a record keyed by [`Acting`], exactly as the tray's menu ids are in
//! `tray.ts`: so an item added here without an act is a compile error there,
//! and every label, id and key stays something a test can read off this value.
//!
//! **Which is the one thing worth pinning.** On Windows and Linux the bar is
//! hidden (ADR-0020) and the menu is the whole of what registers the keystrokes
//! a window showing a web page is expected to answer, so an item dropped from
//! it is a shortcut that silently stops working. And on a Mac the bar is the
//! strip at the top of the screen, which the other two have nothing of: the
//! application menu, File and Help are a Mac's alone, and the Window menu is
//! furniture the platform fills in for itself.
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
  | "about"
  | "services"
  | "hide"
  | "hideOthers"
  | "unhide"
  | "quit"
  | "close"
  | "windowMenu"
  | "help";

/// The items that are Verkstead's own acts rather than the platform's, each
/// the id its item carries — so a press is read back as the thing pressed.
export const ACTS = ["settings", "compose", "guide", "logs", "issue"] as const;

/// One of them.
export type Acting = (typeof ACTS)[number];

/// Where **Verkstead Help** goes: the docs site, in the browser.
export const GUIDE = "https://tobico.github.io/verkstead/";

/// Where **Report an Issue** goes: the issue tracker, in the browser.
export const ISSUES = "https://github.com/tobico/verkstead/issues";

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
  /// Verkstead's own act, where this is one — given its click at the edge.
  readonly id?: Acting;
  /// The keystroke, in Electron's spelling. Only an act carries one: a role's
  /// is the platform's.
  readonly accelerator?: string;
  /// A rule between them.
  readonly type?: "separator";
  /// What a submenu or an act is called. The platform's items are labelled
  /// by their roles.
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

  // The Mac's bar is filled out, and it is the one bar anybody sees: Title Case,
  // the platform's own casing, and `Cmd` rather than `CommandOrControl`, the
  // menu being a Mac's alone. The application menu is written out rather than
  // the `appMenu` role, which Electron fills in and nothing can be added to, so
  // that Settings… can stand where a Mac puts it; every other item in it is
  // still the platform's. Settings… and not Preferences…, which is what macOS
  // 13 renamed it to and what the workbench calls the page.
  if (platform === "darwin") {
    const application: Item = {
      label: "Verkstead",
      submenu: [
        { role: "about" },
        { type: "separator" },
        { id: "settings", label: "Settings…", accelerator: "Cmd+," },
        { type: "separator" },
        { role: "services" },
        { type: "separator" },
        { role: "hide" },
        { role: "hideOthers" },
        { role: "unhide" },
        { type: "separator" },
        { role: "quit" },
      ],
    };

    // Close Window is the platform's `close`: the close policy already hides
    // the window to the Dock on a Mac, the way the red light does.
    const file: Item = {
      label: "File",
      submenu: [
        { id: "compose", label: "New Conversation", accelerator: "Cmd+N" },
        { role: "close" },
      ],
    };

    // The `help` role is what gives a Mac's Help menu its search field.
    const help: Item = {
      role: "help",
      label: "Help",
      submenu: [
        { id: "guide", label: "Verkstead Help" },
        { id: "logs", label: "View Logs" },
        { id: "issue", label: "Report an Issue" },
      ],
    };

    return [application, file, editing, view, { role: "windowMenu" }, help];
  }

  return [editing, view, window];
}
