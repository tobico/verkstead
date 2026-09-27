# Windows

`Verkstead-x86_64.msi` on a Release stops being the Rust tray app's installer
and becomes the Electron one: the packed app installed per user under
`%LOCALAPPDATA%\Programs\Verkstead`, a Start-menu entry beside it, and the CLI's
own directory inside the install on the user's `PATH`, so that `verkstead guide`
works in a terminal opened afterwards and prints the Guide rather than opening a
window. The controls overlay sits at the top-right in the heads' colours; Launch
on Startup is the Run key through Electron's login-item API, taking over the
tray app's own value where a profile has one; and the `desktop-windows` leg
downloads the CLI the matrix already built, packs, installs and asserts — the
upgrade over the package before it among the assertions. The WiX sources for the
Rust msi go, the UpgradeCode carried out of them first.

Most of what this stage is about was written in stages 03 and 04 and has never
been run on Windows — the close policy, the hidden menu bar, the controls
overlay and the Windows arm of the startup registration are all in the tree
already — so the order puts the msi first and everything else behind it:
`startup.ts` answers `nowhere` on an unpackaged run by decision, so nothing
about Launch on Startup can be reached until there is a packed app to reach it
from, and the overlay is worth looking at in the window a downloader gets. The
decisions are in [ADR-0020](docs/adr/0020-electron-desktop.md).

Roadmap stage: [07: Windows](docs/roadmaps/electron-desktop/07-windows.md)

## Tasks

- [ ] 01: The msi, packed from a checkout — [details](01-the-msi.md)
- [ ] 02: The release leg, and the WiX sources retired — [details](02-the-release-leg.md)
- [ ] 03: The overlay, and the window on Windows — [details](03-the-overlay.md)
- [ ] 04: The Run value taken over, and a hidden sign-in start — [details](04-the-run-value.md)
- [ ] 05: The words — [details](05-the-words.md)
