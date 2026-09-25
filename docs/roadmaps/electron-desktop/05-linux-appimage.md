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
  and tray entries in `CONTEXT.md`, and the Linux leg's paragraph in
  `docs/releasing.md`.
- **The trunk stays releasable**: after this stage a Release carries the
  Electron AppImage beside the Rust dmg and msi, and that is accepted.

## Proposed tasks (provisional)

1. **The builder configuration** — electron-builder's AppImage target, the
   CLI as an extra resource found by the main process, the app id, icons and
   desktop entry from `packaging/`. Accepts: a local build from a checkout
   and a cargo-built CLI runs and serves.
2. **The release leg** — `desktop-linux` waits on the CLI matrix, downloads
   `verkstead-linux-x64`, packs, and runs the assertions under Xvfb; the
   glibc-floor check goes with the Rust build. Accepts: the leg is green on a
   dry run of the workflow against the branch.
3. **The Rust AppImage retired** — `tools/build-appimage.sh`, its container
   image and its steps removed from the leg. Accepts: no leg builds the Rust
   tray for Linux.
4. **The words** — adoption, CONTEXT and releasing rewritten for the app on
   Linux, including what a desktop with no tray host now loses (only the
   icon) and what COSMIC proof stage 03 recorded. CONTEXT's Log Directory
   entry says **View Logs** is on the tray menu; what opens the file now is
   that item *or* the Desktop page's button, which is why losing the icon
   loses nothing but the icon. Accepts: nothing in the Linux section names
   `verkstead desktop` or `AppRun`.

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
