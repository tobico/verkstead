# 04. The words

## What to build

Four documents and the README stop describing the Rust tray app on Linux and
describe this one. Only Linux: the Mac and Windows sections are stages 06 and
07's, and the trunk still ships the Rust app on both.

**`docs/adoption.md`'s Linux section** is the one a downloader reads, and most of
it has gone wrong. What it has to carry:

- **What is inside** is the Electron app and the released CLI beside it, rather
  than one binary whose entry point supplies a verb. The same binary is still
  what a session is handed, so the two halves of an ask are one build — that
  invariant is unchanged and is worth keeping in the words.
- **The floor is Electron's**, the number task 03's step reads off the artifact,
  and not the 2.35 a GTK binary was held to. The distributions it names change
  with it.
- **FUSE wants only a `/dev/fuse`** now, the pinned runtime carrying its own
  squashfuse. No libfuse to install, and `--appimage-extract-and-run` is still
  the way past a machine that will not give one.
- **A desktop with no tray host loses the icon and nothing else.** The viewer is
  in the app's own window rather than in a browser tab, so what the icon was the
  only way to is now on the screen anyway, and **View Logs** is on the Desktop
  page as well as the tray menu.
- **The late panel**, which is the accepted risk landing: an app that starts
  before the panel never gets its icon, because Chromium does not register with a
  watcher that arrives afterwards and that run stays iconless. **Show tray icon**
  off and then on again raises another tray, which registers with the watcher
  that is there now — so the switch is the way back as well as the way out.
- **The window has no title bar**, the controls are the platform's own, and on a
  Wayland COSMIC the only one of them drawn is close. A double-click on any pane
  head maximises and a second restores; putting the window away is the minimise
  task 01 put on the hidden menu, or closing it, which keeps Verkstead running in
  the tray.
- **bubblewrap is still the machine's**, and for the reason it always was.

Nothing in the section names `verkstead desktop` or `AppRun`.

**`CONTEXT.md`.** The **Log Directory** entry says **View Logs** is on the tray
menu and on the Desktop page both, which is why losing the icon loses nothing but
the icon — check it says so and leave it if it does. **Startup Registration**
describes an AppImage healing its own entry, which is now the Electron app's
`$APPIMAGE` reading rather than the Rust one's. **Window Decorations** gains what
stage 04 measured on a Wayland COSMIC — the overlay one button wide, Chromium's
doing rather than the compositor's — and the minimise that answers it. Anything
in the file that still describes the Linux tray as the Rust app's four items
becomes this app's three.

**`docs/releasing.md`.** The Linux desktop leg's paragraph, and the paragraph
above it about what holds the legs to their floors, which is written around an
`ubuntu:22.04` container that is gone. Then step 3 of **After the run**, which
tells a human to run the downloaded AppImage with `--help` and expect the tray
app's help back — an Electron app answers nothing of the kind, and what that step
is for is saying the file is what it claims to be. Say what a human should
actually do with it.

**`docs/development.md`.** Its AppImage paragraph describes the unified binary
and the libraries the tray is drawn over, and the `packaging/` paragraph says the
hicolor tree is what `tools/build-appimage.sh` installs. Both become the packed
app and the command task 01 added. Stage 02 has already put right what this file
says about *running* the app.

**The README's Linux sentence** promises "the viewer in your browser and a tray
icon over it", which is no longer what happens: the viewer is in the app's own
window. One sentence, in the paragraph naming the four ways in.

## Acceptance criteria

- [ ] Nothing in `docs/adoption.md`'s Linux section names `verkstead desktop` or
      `AppRun`, and the glibc floor and FUSE requirement it states are the ones
      the leg actually holds.
- [ ] The Linux section carries the late panel and **Show tray icon** as the way
      back, and says what a human does to put the window away.
- [ ] `CONTEXT.md`, `docs/releasing.md`, `docs/development.md` and the README
      describe the app on Linux, and none of them describes the Mac or Windows
      any differently than it did.
