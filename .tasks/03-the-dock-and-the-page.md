# 03. The Dock, Cmd+Q and the Desktop page on a Mac

## What to build

Mostly: run the packed app on a Mac and find out. Stage 03 wrote this platform's
behaviour and reviewed it, and none of it has ever been on a Mac — so this is
one slice of looking at it and closing what is found rather than two of
building, which is why the brief's first two tasks are merged here.

**What is already in the tree**, so the session knows it is confirming rather
than writing:

- `closing.ts` answers `hide` on `darwin` whatever the radio and the tray
  setting say — the platform's own answer, not a position anybody picked.
- `main.ts` registers `activate` and answers one that arrives before the window
  exists by remembering it and bringing the window forward once there is one;
  `before-quit` sets the flag that makes a quit's own close a quit rather than a
  hide; `will-quit` stops the sidecar and lowers the icon.
- `roles.ts` gives a Mac the appMenu and the windowMenu, which is where Quit,
  Hide, Services and Minimize come from; `menu.ts` sets it before the window
  opens.
- `Desktop.tsx` draws no close radio on `darwin`, labels the switch **Show menu
  bar icon**, and draws **View Logs** and **Launch on Startup** on every
  platform.

**What has to be true and has never been seen.** The app has a Dock tile at all
— which is task 01's `LSUIElement` check plus the absence of any
`app.dock.hide()` or accessory activation policy in this codebase; Electron's
default is a regular app, and the menu-bar-only policy ADR-0020 retires was the
Rust app's. Closing the window leaves Verkstead there rather than quitting it.
A click on the Dock tile brings the same window back — the same window, not a
second one, `forward` being restore-show-focus over the window that was hidden.
Cmd+Q quits at once, with no warning, and takes the sidecar with it. And the
application menu is drawn with the app's own name on it rather than Electron's.

**The one thing worth watching.** `window-all-closed` calls `app.quit()`, and on
a Mac the close is always a hide, so it should be reached only on the way out
under Cmd+Q. A Mac that reaches it any other way is an app that quits where
every other Mac app would sit in the Dock, and the log is where that shows.

**And the menu bar icon is a `Tray`.** Electron draws one in the menu bar on
macOS, from the same 192px png `artwork.ts` reads for a Linux panel. A menu bar
wants a template image — a monochrome icon the system inverts for a dark bar —
and a full-colour one drawn at that size is the usual way this looks wrong.
Whether it does, and what to do about it if so, is this task's to look at and
say; the artwork itself is `tools/generate-packaging.sh`'s, so anything new
comes from there.

**Nothing about the close radio may be reachable on a Mac**, including the
warning: `closing` never answers `ask` there, so the dialog cannot be raised,
and the page draws neither the group nor the note under it. Worth checking the
card as well as the pane — it reports what a close comes to, and on a Mac it
says nothing about closing at all.

## Acceptance criteria

- [ ] Closing the window leaves Verkstead in the Dock and a click on the Dock
      tile brings the same window back, with the log saying nothing about
      quitting; the menu bar icon's **Open** does the same.
- [ ] Cmd+Q quits at once with no warning and the log says the sidecar stopped;
      no close radio and no quit warning are drawn or reachable anywhere on the
      Desktop page.
- [ ] **View Logs** on the Desktop page opens the file with the menu bar icon
      switched off, and the icon reads in both a light and a dark menu bar.
