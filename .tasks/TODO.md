# The Mac

`Verkstead-universal.dmg` on a Release stops being the Rust tray app's bundle
and becomes the Electron one: a regular Dock app carrying the two Mac CLI
builds joined with `lipo`, packed by the same electron-builder configuration
the AppImage comes out of. Closing the window leaves Verkstead in the Dock and a
Dock click brings it back; Cmd+Q quits at once; the traffic lights sit in the
head's first row left of the Wordmark; the Desktop page there holds the menu bar
icon, View Logs and Launch on Startup and nothing else; and Launch on Startup is
the platform's own login item, taking over the tray app's launch agent once
where there is one. The `desktop-macos` leg builds, mounts and runs the file it
uploads, and `tools/build-macos-dmg.sh` with its launcher script goes.

Most of what this stage is about was written in stage 03 and has never been run
on a Mac — the close policy, the Dock activation, the application menu and the
Desktop page's Mac shape are all in the tree already — so the order here puts the
dmg first and everything else behind it: `startup.ts` refuses a registration on
an unpackaged run, so nothing about Launch on Startup can even be reached until
there is a packed app to reach it from. The decisions are in
[ADR-0020](docs/adr/0020-electron-desktop.md).

Roadmap stage: [06: The Mac](docs/roadmaps/electron-desktop/06-macos.md)

## Tasks

- [x] 01: The universal dmg, packed from a checkout — [details](01-the-universal-dmg.md)
- [x] 02: The release leg, and the Rust dmg retired — [details](02-the-release-leg.md)
- [x] 03: The Dock, Cmd+Q and the Desktop page on a Mac — [details](03-the-dock-and-the-page.md)
- [x] 04: The traffic lights in the head's first row — [details](04-the-traffic-lights.md)
- [x] 05: The launch agent taken over, and a hidden login start — [details](05-the-launch-agent.md)
- [ ] 06: The words — [details](06-the-words.md)
