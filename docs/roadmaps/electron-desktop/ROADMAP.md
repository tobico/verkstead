# Electron desktop roadmap

Verkstead's desktop app becomes an Electron application: a window of its own
over the workbench, the headless `verkstead` CLI run beside it as a sidecar
(`verkstead serve --desktop`), and the tray, the dialogs and the startup
registration reached through one toolkit rather than three. The server still
listens on `127.0.0.1:8422`, for the window and for a phone over `tailscale
serve` alike, and the headless CLI ships exactly as it does today. Closing the
window is a three-way choice on Windows and Linux and the Dock's own behaviour
on a Mac, settled on a new **Desktop** page at the top of the settings that
only the app draws. The decisions and their why are in
[ADR-0020](../../adr/0020-electron-desktop.md), which supersedes
[ADR-0012](../../adr/0012-desktop-tray-binary.md); the terms are in
[CONTEXT.md](../../../CONTEXT.md), which each stage updates as its piece
lands, as does [development.md](../../development.md) — a fifth of which is
about the crate this roadmap retires, and which goes wrong at stage 02 rather
than at the end.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

Sequential from 01 to 05, then a fork: 06 and 07 each need 05 and run in
either order, and 08 needs both. The trunk stays releasable throughout — the
Rust tray app keeps shipping on each platform until the stage for that
platform takes its release leg, and 08 retires it only once none is left.

## Stages

- [x] 01: The sidecar flag — [brief](01-sidecar-flag.md)
- [ ] 02: The Electron shell — [brief](02-electron-shell.md) *(in progress: `roadmaps/electron-desktop/02-electron-shell`)*
- [ ] 03: Tray, close policy and the Desktop page — [brief](03-tray-and-desktop-page.md)
- [ ] 04: Client-side decorations — [brief](04-decorations.md)
- [ ] 05: The Linux AppImage — [brief](05-linux-appimage.md)
- [ ] 06: The Mac — [brief](06-macos.md)
- [ ] 07: Windows — [brief](07-windows.md)
- [ ] 08: Retiring the Rust tray — [brief](08-retiring-the-rust-tray.md)
