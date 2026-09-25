# The Electron shell

`pnpm start` in a new top-level `desktop/` project, from the nix dev shell on
Linux, starts the headless `verkstead` beside it as `serve --desktop`, waits for
its health, reads the **Workbench Key** out of the **Data Directory** and opens
one window on the workbench logged in. A second launch brings that window
forward; a foreign listener on `127.0.0.1:8422` is a dialog and an exit; the
server ending quits the app; closing the window quits it too, for now. The
sidecar's stdout lands in `verkstead.log` under the **Log Directory** beside the
app's own lines, with the byte-order mark and the roll the Rust app gave it.
Links off the workbench open in the system browser, the window's size and
position come back next run, and the menu bar is hidden with its shortcuts kept.

Electron only, over the sidecar stage 01 built. Nothing on the wire changes —
the viewer is loaded exactly as it is served — and the Rust tray app goes on
shipping until stages 05 to 08 take each platform's release leg. The tray, the
close policy and the Desktop page are stage 03's, the frameless window is stage
04's, and packaging is 05's onwards. The decisions are in
[ADR-0020](docs/adr/0020-electron-desktop.md), which supersedes
[ADR-0012](docs/adr/0012-desktop-tray-binary.md).

Roadmap stage: [02: The Electron shell](docs/roadmaps/electron-desktop/02-electron-shell.md)

## Tasks

- [x] 01: The toolchain moves to nixos-26.05 — [details](01-the-toolchain-moves.md)
- [x] 02: The project, the shell and the sidecar — [details](02-the-project-and-the-sidecar.md)
- [x] 03: The window, logged in — [details](03-the-window-logged-in.md)
- [x] 04: Lifecycle — [details](04-lifecycle.md)
- [ ] 05: The log file — [details](05-the-log-file.md)
- [ ] 06: Window manners — [details](06-window-manners.md)
- [ ] 07: The words — [details](07-the-words.md)
