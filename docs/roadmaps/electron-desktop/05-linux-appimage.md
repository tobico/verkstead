# 05. The Linux AppImage

## Goal

`Verkstead-x86_64.AppImage` on a Release is the Electron app, carrying the
static musl `verkstead` the same run's CLI leg built. The `desktop-linux`
release leg builds it with electron-builder, runs the file that is uploaded
under Xvfb, and asserts it serves a document naming the viewer's bundle, that
the binary inside answers `ask`, and that its log says the window and the
tray came up. The Rust AppImage script and its leg are gone; the Linux
section of the adoption docs and the CONTEXT.md terms describe the app.

## Decisions in force

- **The sidecar is the released CLI artifact** ([ADR-0020], Set 848 Q14):
  the static musl binary, downloaded from the CLI leg of the same run rather
  than built again. A glibc build of its own was rejected — nothing in the
  sidecar links a toolkit now, so the only floor is Electron's.
- **electron-builder, an x86_64 AppImage, unsigned** (Q15, Q16a). arm64 Linux
  was offered and not taken.
- **Each leg proves the artifact it uploads** ([releasing.md]): the three
  assertions above, bounded so a dialog nobody dismisses cannot hold a runner.
- **Sessions need the host's bubblewrap and FUSE** as before; what changes is
  what is inside the file.
- **Records are rewritten by the stage that ships each platform** (Q19): the
  Linux section of `docs/adoption.md`, the Log Directory, Startup Registration
  and tray entries in `CONTEXT.md`, the Linux leg's paragraph in
  `docs/releasing.md`, and that platform's artifact in `docs/development.md` —
  which is the document a developer reads to build one, and which stage 02
  has already put right about running the app.
- **The trunk stays releasable**: after this stage a Release carries the
  Electron AppImage beside the Rust dmg and msi, and that is accepted.

## What stage 03 found on COSMIC

The run ADR-0020 asked for, and the one that discharges the Linux tray risk:
COSMIC 1.2.0 — `cosmic-comp`, `cosmic-panel` and its status-area applet —
with the app on Electron 43.1.0 beside a stand-in sidecar. Every gesture below
was a real pointer click into the compositor, and what the panel and the app
said to each other was read off the session bus. The session was a nested one
on software rendering rather than a machine booted into COSMIC, so what it
proves is the protocol between the app and COSMIC's own tray host rather than
anything about the graphics stack.

- **The icon, its menu and all three items are good.** The packaging artwork
  appears in the status area beside COSMIC's own applets; a right click draws
  **Open**, **View Logs** and **Quit** in that order, from the app's menu
  rather than a copy of it. **Open** brought a closed-to-tray window back,
  **View Logs** opened this run's log file, and **Quit** took the app and its
  sidecar without asking — including with **When the window is closed** set to
  *ask before quitting*, which is Q7b proven rather than assumed.
- **A left click on the icon is an `Activate`**, after a
  `ProvideXdgActivationToken`, which is the gesture Electron raises `click`
  on — so on COSMIC the icon opens the window. **Open** being first on the
  menu was not needed here; it stays for the desktops that answer a left click
  with the menu instead.
- **An app that starts before the panel never gets its icon.** With no
  `StatusNotifierWatcher` on the bus the app comes up and says the icon is in
  the tray, and when `cosmic-panel` arrives afterwards Chromium does not
  register: the watcher's `RegisteredStatusNotifierItems` stays empty for as
  long as that run lasts, though the applet's own `RegisterStatusNotifierHost`
  and the `NameOwnerChanged` for the watcher's name both go past on the bus.
  This is the accepted risk landing, and it is the Linux words' to describe.
- **The repair is already in the product**: **Show tray icon** off and then on
  again destroys that tray and raises another, which registers with the
  watcher that is there now and puts the icon on the panel. So the switch is a
  way back as well as a way out, and that is what the Linux section should say
  — a run that lost its icon loses nothing else, the window being untouched and
  **View Logs** being on the Desktop page.
- **The log file is `text/x-log`**, which is what `shell.openPath` hands to
  `xdg-open`. A desktop with an association for that type opens it. A desktop
  with one only for `text/plain` does not: `xdg-open` does not follow the
  subclass the way `gio open` does, and it exits 0 having done nothing, so the
  app reports success and the human sees no log. Worth settling when the
  packaged app's desktop entry is written.

## What stage 04 found on COSMIC

The run that says what the frameless window is like on this desktop, and the
one that discharges the risk ADR-0020 took on the controls overlay: the same
nested COSMIC 1.2.0 session stage 03 used — `cosmic-comp` under Xvfb on
software rendering — with the finished app, the real `verkstead` sidecar and a
workbench a pointer was driven around by hand. The app came up as a **native
Wayland client**, which is the path a real COSMIC session takes: that session
sets `XDG_SESSION_TYPE=wayland` and the pinned Electron picks Wayland by
itself, with no ozone flag from us. As in stage 03, what a nested session on
llvmpipe proves is what the app and the compositor say to each other rather
than anything about the graphics stack.

**Everything this stage built is right there.**

- **No title bar.** Chromium asks COSMIC for client-side decorations
  (`zxdg_toplevel_decoration_v1.set_mode(1)`) and COSMIC draws nothing but its
  own thin focus border around the window. The page's own head is the top of
  the window.
- **The overlay lands, 32 px wide, and as tall as the band the page pushed** —
  75 px at a sixteen-pixel root, which is the number the page computes and the
  app logs. `getTitlebarAreaRect()` reads `{ x: 0, width: innerWidth - 32 }`,
  the frame carries `--controls-right: 32px`, and the head at that edge is
  padded by exactly that: the details head in two panes, and the single pane
  of a narrow window, while the sidebar's head — which is not at that edge —
  is padded by nothing. No control ended up under the controls in any layout
  that was drawn.
- **Every head moves the window.** The sidebar's Wordmark, the details head and
  the setup page's bare drag bar each dragged it by exactly the pointer's
  delta. The way back out of a pane — a button across the whole width of the
  head — does not: a drag on it left the window where it was, which is the
  `no-drag` exception holding on a real compositor rather than in jsdom. The
  Settings gear pressed and navigated with the window unmoved.
- **A flip of the scheme recolours the overlay with no restart.** The machine
  going dark had the page push `#171614` with `#ece7e0` marks, the app log the
  push, and the close button turn from a dark mark on light paper into a light
  mark on dark.
- **A double-click on any pane head maximises, and a second one restores.**

**And the overlay is one button wide — which is Chromium's doing rather than
COSMIC's.**

- Under Wayland the overlay is 32 px and a close button alone. The same app, on
  the same session, over Xwayland is 96 px and the usual three. So the missing
  pair follows the platform Chromium picked and not the desktop.
- **COSMIC is not refusing them.** Its `xdg_wm_base` is at version 7 and the
  toplevel's `wm_capabilities` carries all four — window menu, maximize,
  fullscreen, minimize. Turning COSMIC's own **show_minimize** and
  **show_maximize** toolkit settings on and restarting the compositor left the
  overlay one button wide, and so did writing a GTK decoration layout into the
  app's config: neither is the lever.
- **Minimising works when the app asks for it.** `minimize()` sends
  `set_minimized`, COSMIC takes the window off the screen, and the way back is
  the app's own — the tray's **Open**, which is the restore, show and focus of
  `forward()`, put it back.
- **Nothing COSMIC offers replaces the two buttons.** A right-click on a head
  sends `xdg_toplevel.show_window_menu` and COSMIC 1.2.0 draws nothing for it.
  Its shipped shortcut defaults bind Maximize to Super+M, Close to Super+Q and
  Alt+F4, and Fullscreen to Super+F11 — and bind **no Minimize at all**. The
  action is in the compositor's vocabulary, so a human can bind a key to it in
  COSMIC's settings, but nothing is bound out of the box. (The compositor's own
  window switcher never drew in this nested session, so what Alt+Tab does with
  a minimised Verkstead on a real machine went untested, as did whether a real
  COSMIC's own portal reports a button layout that changes any of this.)

**So what is left wanting is minimise, and it is this stage's to settle rather
than the last one's.** Maximise is reachable from the app's own chrome — a
double-click on any head — so the one thing a COSMIC user of Verkstead has no
gesture for is putting the window away. Whether that is a loss or COSMIC's own
convention was not settled here — what a COSMIC-decorated window of its own
draws was not measured. Three answers are open and none of them is free:
accept it and say so in the Linux words, the tray being the way back and
closing already keeping the app running; force the X11 path for the packaged
app, which brings the three buttons back and gives up Wayland-native rendering
on every Wayland desktop to fix one; or give the app a minimise of its own —
a tray item, a keystroke, or a control the page draws, which is the custom
control ADR-0020 turned down for the overlay.

## Proposed tasks (provisional)

1. **The builder configuration** — electron-builder's AppImage target, the
   CLI as an extra resource found by the main process, the app id, icons and
   desktop entry from `packaging/`. Where the resource lands is a directory of
   the CLI's own rather than beside the launcher: stage 07 puts that directory
   on Windows' `PATH` and cannot put the launcher's there, so one layout
   serves all three platforms. Accepts: a local build from a checkout and a
   cargo-built CLI runs and serves.
2. **The release leg** — `desktop-linux` waits on the CLI matrix, downloads
   `verkstead-linux-x64`, packs, and runs the assertions under Xvfb; the
   glibc-floor check goes with the Rust build. Accepts: the leg is green on a
   dry run of the workflow against the branch.
3. **The Rust AppImage retired** — `tools/build-appimage.sh`, its container
   image and its steps removed from the leg. Accepts: no leg builds the Rust
   tray for Linux.
4. **The words** — adoption, CONTEXT, releasing and development rewritten for
   the app on Linux, including what a desktop with no tray host now loses
   (only the icon) and what COSMIC proof stage 03 recorded. CONTEXT's Log
   Directory entry says **View Logs** is on the tray menu; what opens the file
   now is that item *or* the Desktop page's button, which is why losing the
   icon loses nothing but the icon. development's build list drops
   `tools/build-appimage.sh` and its AppImage paragraph describes the packed
   app. The late-panel case above belongs here too: the icon is lost to a
   panel that arrived after the app, and **Show tray icon** off and on again
   is what brings it back. The window's own decorations belong here as well:
   the title bar is gone, the controls are the platform's own, and on a
   Wayland COSMIC the only one of them drawn is close — a double-click on any
   pane head maximises, and whatever this stage settles about minimise is what
   the Linux section has to say about putting the window away. Accepts:
   nothing in the Linux section names `verkstead desktop` or `AppRun`.

## Re-verify at start

- Stages 02 to 04 landed and the app runs from the dev shell.
- The CLI matrix's artifact names and the `desktop-*` prefix the publish job
  keys on, in `release.yml`.
- Whether the musl CLI still serves and asks from inside a mounted AppImage
  path, which is what the Sandbox binds.
- The AppImage runtime electron-builder pins, against the FUSE note in the
  adoption docs.

[ADR-0020]: ../../adr/0020-electron-desktop.md
[releasing.md]: ../../releasing.md
