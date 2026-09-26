//! The menu bar, which is hidden everywhere it can be and never absent.
//!
//! **Hidden on Windows and Linux, kept on a Mac** (ADR-0020). A menu bar drawn
//! across the top of this window would be a browser's chrome over a workbench
//! that already has its own navigation, and the app is meant to read as one
//! thing rather than as a page inside a program. A Mac's menu is not in the
//! window at all — it is the strip at the top of the screen that says which
//! application is in front, and an app without one is an app whose Quit, Hide
//! and Services the human cannot find.
//!
//! **And hidden is not gone.** Copy, paste, select all, the zoom steps and
//! their reset, reload and the developer tools are the keystrokes a window
//! showing a web page is expected to answer, and on Windows and Linux it is the
//! menu that registers every one of them: an app with no menu at all is an app
//! where Ctrl+C does nothing. So the menu is built and set, and the *bar* is
//! what is hidden — see [`open`](./window.js), which does the hiding. Alt still
//! brings it down, which is the platform's own answer for a hidden menu bar and
//! one less thing to explain.
//!
//! Everything here is a role rather than a handler. A role is the platform's own
//! item — its label in the machine's language, its accelerator in the
//! platform's spelling, and its behaviour written by people who know what
//! Cmd+Z means in a text field on a Mac.

import { Menu, type MenuItemConstructorOptions } from "electron";

/// Set the application menu, from which every shortcut in it is registered.
///
/// Called once, after the app is ready and before the window it applies to is
/// opened, so that the window is never briefly a window whose keystrokes do
/// nothing.
export function shortcuts(platform: NodeJS.Platform): void {
  Menu.setApplicationMenu(Menu.buildFromTemplate(template(platform)));
}

/// What the menu holds on this platform.
function template(platform: NodeJS.Platform): MenuItemConstructorOptions[] {
  const editing: MenuItemConstructorOptions = {
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

  const view: MenuItemConstructorOptions = {
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

  // The Mac's own furniture: the application menu it cannot do without, and the
  // Window menu that holds Minimize and Zoom — both of which the platform fills
  // in for itself.
  return platform === "darwin"
    ? [{ role: "appMenu" }, editing, view, { role: "windowMenu" }]
    : [editing, view];
}
