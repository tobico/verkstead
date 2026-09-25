# 06. Window manners

## What to build

The three things that make the window an app's rather than a browser's: links
off the workbench open in the system browser, the window's size and position come
back next run, and the menu bar is hidden with its shortcuts kept (ADR-0020).

**The window never navigates away from localhost.** Every navigation to anywhere
else — a gist, a pull request, a documentation link — is handed to the system
browser and the window stays where it was, and a page that asks for a new window
or a new tab is the same answer rather than a second Electron window. Only the
schemes a browser should be handed get handed over; anything else is refused
rather than passed to the platform's opener.

**Bounds are remembered per machine, in Electron's own user data** — a state
file of the app's, not the server's `config.yaml` and nothing on the wire: where
a window sits is a fact about the machine in front of the human, which is the
same split the per-device push switch already made. A window whose remembered
place is off every display this machine now has comes back visible rather than
somewhere nobody can reach it.

**The menu bar is hidden on Windows and Linux**, and the shortcuts it would have
carried are not: copy, paste, select all, the zoom steps and reset, reload, and
devtools all still work. A Mac keeps the application menu it cannot do without —
written here as the platform's arm and proven in stage 06.

The window is still the platform's own decorated window here; the frameless one
with the controls overlay is stage 04's.

**What vitest covers**: the rule that decides what is the window's own and what
goes to the browser, over a list of URLs including the loopback in its several
spellings; and the remembered bounds fitted to a set of displays it is handed,
including the one where the display is gone.

## Acceptance criteria

- [ ] A link off the workbench opens the system browser and the window is still
      on the workbench, and a target that would open a new window does the same.
- [ ] A moved and resized window comes back where it was on the next run, and one
      whose display has gone comes back on a display that is there.
- [ ] Copy, paste, zoom, reload and devtools work with no menu bar drawn on
      Linux.
