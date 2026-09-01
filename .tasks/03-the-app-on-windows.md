# 03. The desktop app on Windows

## What to build

`verkstead-desktop.exe`: the same app the AppImage and the dmg carry, drawn on
Windows, and a portable single file that runs from wherever it sits. No
installer and no MSI (ADR-0012).

Three arms, each of which the crate already has a shape and a seam for:

- **The loop.** `toolkit` holds a `compile_error!` saying the Windows arm is
  this stage's. The obligations are the ones its documentation already spells
  out: started on the main thread, blocking it, ended both by **Exit** off the
  menu — which arrives on the loop's own thread — and by the server stopping
  underneath the app, which arrives on another and has to ask rather than do.
- **The dialogs.** The two things this binary draws that carry words: the
  address it could not take, and the failure that stopped it. Drawn on the
  loop's own thread, as they are on both other platforms.
- **Launch on Startup.** The Run key, which is another arm of the shape
  `startup` already has — an entry that is there or is not, read afresh every
  time the menu is drawn. Rewritten at every launch while it is there, which is
  exactly what a portable exe that has been moved needs, and never written for
  a machine that never asked.

The rest is already built and should need nothing: the menu and its four items,
the icon in the tray, and the double-click that runs **Open** — `tray-icon`
reports one on Windows and the existing binding already handles it. The log
goes where stage 01 said Windows keeps one, under `%LOCALAPPDATA%`.

**The exe carries its own icon**, so Explorer and the taskbar draw Verkstead
rather than a default. `tools/generate-packaging.sh` says stage 05 adds the
Windows `.ico` beside the `.icns`; it is generated from the one piece of
artwork and committed, like everything else that directory holds.

The human has a Windows machine and will smoke this by hand, so what the runner
has to prove is narrower: that the exe starts, serves, and says in its own log
that an icon went up.

## Acceptance criteria

- [ ] On Windows the icon appears in the tray, double-click and **Open** both
      put the viewer in front, **View Logs** opens the file, and **Exit** stops
      the server — and the log carries the line a release leg can read to know
      a tray came up
- [ ] Ticking **Launch on Startup** writes a Run key naming the running exe and
      unticking removes it; an exe launched from a new directory rewrites the
      registration it left behind, and a machine nobody registered stays
      unregistered — covered by tests where the libraries allow
- [ ] A `127.0.0.1:8422` somebody else is already listening on draws the dialog
      and exits rather than serving
- [ ] `tools/generate-packaging.sh` writes the `.ico`, it is committed, and the
      built exe carries it
