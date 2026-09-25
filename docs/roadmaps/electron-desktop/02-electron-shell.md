# 02. The Electron shell

## Goal

`pnpm start` in a new top-level `desktop/` project, from the nix dev shell on
Linux, starts the headless `verkstead` beside it as `serve --desktop`, waits
for its health, reads the Workbench Key out of the Data Directory and opens
one window on the workbench logged in. A second launch brings that window
forward; a foreign listener on `127.0.0.1:8422` is a dialog and an exit; the
server ending quits the app; closing the window quits it too, for now. The
sidecar's stdout lands in `verkstead.log` under the Log Directory beside the
app's own lines. Links off the workbench open in the system browser, the
window's size and position come back next run, and the menu bar is hidden with
its shortcuts kept. CI lints, typechecks and tests the project.

## Decisions in force

- **A top-level `desktop/` pnpm project, TypeScript main and preload, built
  by electron-builder** ([ADR-0020], Set 848 Q15). Not a package inside
  `web/`: it consumes a built binary rather than sharing a toolchain. Forge
  was rejected for electron-builder's three targets in one configuration.
- **Electron holds the key the way the tray app did** (Set 845 Q3): read from
  `workbench.key` once health answers, the window opened on the login link,
  the file read again on a 401 of the window's own frame. Not a stdout line
  to parse and not a key handed in.
- **Fixed port, single-instance lock, dialog and exit on a foreign listener**
  (Q4), as [ADR-0012] decided and for the same reasons; a free port was
  rejected again.
- **The app owns the log file** (Q5): the sidecar keeps logging to stdout, the
  app writes those lines beside its own into `verkstead.log`, with the
  byte-order mark and the roll at 4 MiB to `.1` that the Rust app gave it, in
  the Log Directory CONTEXT.md names.
- **The window** (Set 846 Q6): menu bar hidden on Windows and Linux with copy,
  paste, zoom, reload and devtools still wired; every navigation away from
  localhost goes to the system browser; bounds remembered in Electron's own
  state file, per machine.
- **The server ending is the app quitting**, carried over from the tray app.
- **nixpkgs' Electron in the dev shell** (Set 848 Q18) for running the app
  locally; electron-builder's downloads stay CI's.
- **Proof** (Q17): vitest over the main process's pure parts. No driven
  end-to-end suite.
- **Nothing on the wire changes** — the viewer is loaded as it is served.

## Proposed tasks (provisional)

1. **The project and the sidecar** — `desktop/` with its lint, typecheck and
   test scripts; the main process finds the CLI (a dev-time path to a cargo
   build, later the bundled one), starts `serve --desktop` with the platform
   Data Directory, and waits for health. Accepts: the sidecar is a child that
   dies with the app; a missing binary is a dialog naming the path.
2. **The window, logged in** — the key read from the Data Directory, the
   window opened on the login link, the 401 re-read. Accepts: the workbench
   draws with no 401; a key reset from another browser has the window back
   in after one reload.
3. **Lifecycle** — single-instance lock, the taken-port dialog before the
   sidecar is started, the server ending quitting the app, close quits.
   Accepts: a second launch focuses the first; a listener planted on 8422
   exits non-zero after the dialog; killing the sidecar ends the app.
4. **The log file** — sidecar stdout and the app's own lines into
   `verkstead.log`, BOM, roll, `.1`. Accepts: the file rolls at the bound;
   the startup line in it carries no key.
5. **Window manners** — external links out, bounds remembered, menu hidden
   with shortcuts. Accepts: a gist link opens the system browser and the
   window stays on localhost; a moved window comes back where it was.
6. **The dev shell and CI** — nixpkgs' electron in `flake.nix`; a `desktop`
   job in `ci.yml` beside `viewer`. Accepts: `pnpm start` runs from a fresh
   shell; CI is green on the branch that adds the project.
7. **The words** — `docs/development.md`'s account of running the app, which
   from this stage is `pnpm start` in `desktop/` rather than
   `cargo run -p verkstead-cli -- desktop --data-dir .`, and the paragraph on
   what the app puts on the screen, which is a window here and a tray in 03.
   The packaging and toolkit sections are stages 05 to 08's. Accepts: nothing
   in the running-it section tells a developer to start the tray app.

## Re-verify at start

- Stage 01 landed: `serve --desktop` exists and redacts the line.
- The Log Directory and Data Directory defaults are still the platform's
  (`verkstead_server::platform`), and `workbench.key` is still the file's name.
- Whether `/api/v1/health` is still the open route to wait on.
- The flake's node and pnpm pins, and whether nixpkgs' Electron major matches
  the `electron` the project pins — a mismatch is a dev-only Electron that
  differs from the packed one.

[ADR-0020]: ../../adr/0020-electron-desktop.md
[ADR-0012]: ../../adr/0012-desktop-tray-binary.md
