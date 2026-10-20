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
//! their reset, reload, the developer tools and the minimise this app has no
//! other gesture for on a Wayland desktop are the keystrokes such a window is
//! expected to answer, and on Windows and Linux it is the menu that registers
//! every one of them: an app with no menu at all is an app where Ctrl+C does
//! nothing. So the menu is built and set, and the *bar* is what is hidden — see
//! [`open`](./window.js), which does the hiding. Alt still brings it down, which
//! is the platform's own answer for a hidden menu bar and one less thing to
//! explain.
//!
//! Everything in it is a role or an id rather than a handler, and which a
//! platform gets is [`roles.ts`](./roles.js)'s — a value, and one vitest can be
//! asked about. What is left here is the handing over, which is a call on
//! `electron` and the only reason this file is at the wall in
//! `eslint.config.js`, and the one step that makes an id a click — and the
//! greying of one, which is decided in `where.ts` and only applied here. The
//! handover is also what checks that template: a role Electron does not have
//! fails to typecheck on the line below, and an id with no act fails at `acts`.

import { Menu, type MenuItemConstructorOptions } from "electron";

import { type Acting, type Item, roles } from "./roles.js";

/// Set the application menu, from which every shortcut in it is registered.
///
/// Called once, after the app is ready and before the window it applies to is
/// opened, so that the window is never briefly a window whose keystrokes do
/// nothing. So an act reads whatever it needs at the press rather than being
/// handed a window here — there is none yet.
///
/// Returns the menu it set, which is what [`grey`] is later handed.
export function shortcuts(platform: NodeJS.Platform, acts: Record<Acting, () => void>): Menu {
  const clicking = (item: Item): MenuItemConstructorOptions => ({
    ...item,
    ...(item.id === undefined ? {} : { click: acts[item.id] }),
    ...(item.submenu === undefined ? {} : { submenu: item.submenu.map(clicking) }),
  });

  const menu = Menu.buildFromTemplate(roles(platform).map(clicking));
  Menu.setApplicationMenu(menu);
  return menu;
}

/// Grey and enable the menu's acting items, by id, as `answer` says — the
/// answer being [`enabled`](./where.js)'s, worked out where vitest can ask about
/// it. An id the menu has not got is a platform whose bar carries no such item,
/// and is passed over.
export function grey(menu: Menu, answer: Partial<Record<Acting, boolean>>): void {
  for (const [id, on] of Object.entries(answer)) {
    const item = menu.getMenuItemById(id);
    if (item !== null) {
      item.enabled = on;
    }
  }
}
