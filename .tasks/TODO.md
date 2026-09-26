# The Linux AppImage

`Verkstead-x86_64.AppImage` stops being the Rust tray app and becomes the
Electron one, carrying the static musl `verkstead` that the same release run's
CLI leg built rather than a binary compiled a second time. electron-builder
packs it from a configuration that stages 06 and 07 will fill their own targets
into; the `desktop-linux` release leg waits on the CLI matrix, downloads that
binary, packs, and then proves the very file it uploads — it serves a document
naming the viewer's bundle, the binary inside answers `ask`, and its own log
says the window and the tray came up. `tools/build-appimage.sh` and everything
that only it wanted go with it.

Two things follow from the swap that are not the packing. The server carries a
reading of `$APPDIR` written for the Rust AppImage — a launcher and a bind of
the libraries beside the running image — and an electron-builder AppImage
satisfies every condition of it in front of a statically linked sidecar that
wants none of it, so that reading goes here. And the Linux words are rewritten
for the app: its floor is Electron's rather than the 2.35 a GTK binary was held
to, what FUSE the pinned runtime wants is not what the old one wanted, a desktop
with no tray host now loses the icon and nothing else, and the window has no
title bar. The decisions are in
[ADR-0020](docs/adr/0020-electron-desktop.md).

Roadmap stage: [05: The Linux AppImage](docs/roadmaps/electron-desktop/05-linux-appimage.md)

## Tasks

- [x] 01: The builder configuration, and an AppImage from a checkout — [details](01-the-builder-configuration.md)
- [ ] 02: What the server reads of an AppImage — [details](02-what-the-server-reads.md)
- [ ] 03: The release leg, and the Rust AppImage retired — [details](03-the-release-leg.md)
- [ ] 04: The words — [details](04-the-words.md)
