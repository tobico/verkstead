# 01. The builder configuration, and an AppImage from a checkout

## What to build

electron-builder joins `desktop/`, with one configuration serving all three
platforms and only the Linux target filled in — stages 06 and 07 add theirs to
this same file. Beside `pnpm start` there is a command that packs, taking the
CLI it is to carry from wherever the developer says it is, so that a checkout
with a `cargo build --release`d `verkstead` in it can produce
`Verkstead-x86_64.AppImage` and run it.

**The CLI goes in a directory of its own, `cli/` under the pack's resources.**
Not beside the launcher: the root of the install holds a launcher
electron-builder names for the product, Windows resolves `PATH` without regard
to case, and a `verkstead` there would answer a lookup with the window instead
of the guide (ADR-0020). Stage 07 puts that directory on Windows' `PATH` and
cannot put the launcher's there, so one layout serves all three. The app already
looks for it there and needs no change to find it — see `cli.ts`, whose packed
arm was written for this task to satisfy. The launcher's own name must not be
`verkstead` for the same reason.

**The artifact's name is written down rather than defaulted.** electron-builder
names an AppImage for the product, the version and the architecture; `publish`
names `Verkstead-x86_64.AppImage`, the documentation spells it, and an install
line matches it — so the configuration says it.

**The static AppImage runtime, pinned — `toolsets: { appimage: "1.0.3" }`**
(settled in planning, Set 885 Q3). With nothing said, electron-builder 26.16
builds against `appimage-12.0.1`, whose runtime wants **libfuse2** — which
Ubuntu 24.04, Debian 13 and current Fedora do not install, so the file would
refuse to mount on a current desktop with *"Cannot mount AppImage, please check
your FUSE setup"*. The static runtime links its own squashfuse and wants only a
`/dev/fuse` to open. It also compresses with zstd rather than xz, and it does
not add `--no-sandbox` to the desktop entry the way the legacy path does.

**The desktop entry is configuration, not `packaging/`'s file.**
electron-builder writes its own entry from the app's name, comment, categories
and keywords, with an `Exec` of its own that no configuration replaces — so
what carries over from `packaging/` is the hicolor icon tree, read as a
directory, and the entry's *fields*. `tools/generate-packaging.sh` goes on
writing what the Rust app still needs until stage 08 retires it.

**Where the Electron that gets packed comes from is this task's to settle.**
The `electron` package's install script is denied in
`desktop/pnpm-workspace.yaml` — nothing in the project has wanted a downloaded
runtime, `pnpm start` running the dev shell's instead — so `node_modules` holds
no runtime for electron-builder to pack. Either the pack fetches its own by the
pinned version, or the denial is lifted for the pack alone; what it must not do
is pack the dev shell's Electron, which is a different patch version from the
one `package.json` pins.

**And the window gains a minimise** (Set 885 Q2). Stage 04 measured the
controls overlay at one button on a Wayland COSMIC — a close and nothing else,
which is Chromium's doing rather than the compositor's — and COSMIC binds no
minimise key of its own out of the box, so a human there has no gesture for
putting the window away. The menu bar is already hidden on Linux and Windows
with its shortcuts kept, and the app's menu has no window role on those
platforms yet: a minimise role on that hidden menu draws nothing, costs nothing
and carries the platform's own accelerator. Not a control the page draws, which
ADR-0020 turned down, and not the X11 path, which would give up Wayland-native
rendering on every Wayland desktop to fix one.

## Acceptance criteria

- [ ] An AppImage packed in a checkout, against a release-built CLI, runs: it
      serves the workbench in its own window, and its log names both the window
      opening and the icon going into the tray.
- [ ] The binary inside the mounted image answers `ask --help`, and the launcher
      at the root of the image is not called `verkstead`.
- [ ] The file is called `Verkstead-x86_64.AppImage`, it mounts on a machine with
      no libfuse2 installed, and the configuration names no Linux-only step that
      stages 06 and 07 cannot extend with a target of their own.
- [ ] The minimise role is on the hidden menu, the desktop suite pins it for the
      two platforms that get one, and it puts the window away on a real
      compositor.
