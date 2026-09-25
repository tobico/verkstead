# 07. Windows

## Goal

`Verkstead-x86_64.msi` on a Release installs the Electron app per user under
`%LOCALAPPDATA%\Programs`, with a Start-menu entry and the install directory
on the user's PATH, so `verkstead guide` works from a fresh terminal. The
controls overlay sits at the top-right in the heads' colours; Launch on
Startup is the Run key through the login-item API; the `desktop-windows` leg
installs the msi and asserts the install, the record under HKCU, the
Start-menu entry, the PATH, and the three assertions every leg makes. The
WiX sources for the Rust msi are gone.

## Decisions in force

- **An msi, through electron-builder's WiX target** ([ADR-0020], Set 848
  Q16): the human kept the msi over the NSIS installer recommended.
- **The install directory stays on the user's PATH** (Set 850 Q20): a WiX
  fragment of our own in the build, because the target does not do it alone.
  The adoption docs promise it and the leg asserts it.
- **Per user, unsigned**, as before: the SmartScreen steps in the adoption
  docs kept.
- **The overlay and its colours** are stage 04's code, proven here.
- **Launch on Startup through the login-item API** (Set 846 Q9a), which is
  the Run key underneath; hidden start when the tray is shown.
- **Sessions on ConPTY and the AppContainer are untouched**: the CLI inside
  is the same, and the named pipe a container asks through is the server's.
- **The Windows Installer version stays three numbers**, so the package must
  still allow an upgrade from a version reading the same as its own
  ([releasing.md]).

## Proposed tasks (provisional)

1. **The overlay on Windows** — proven with the stage-04 code; snap layouts
   work. Accepts: no control under the overlay in any pane count.
2. **Login item** — the Run key arm through the API, hidden start. Accepts:
   the value appears and disappears with the box and names the app's own
   path.
3. **The msi** — electron-builder's WiX target, per-user, the PATH fragment,
   the Start-menu entry, the same-version upgrade rule. Accepts: a local build
   installs under the profile; `verkstead guide` runs from a new terminal.
4. **The leg** — `desktop-windows` downloads `verkstead-windows-x64.exe`,
   packs, installs, and asserts; `tools/verkstead.wxs` and
   `tools/build-windows-msi.sh` retired. Accepts: the leg is green.
5. **The words** — the Windows sections of adoption and releasing rewritten.
   Accepts: nothing names the shim.

## Re-verify at start

- Stage 05 landed; whether 06 has, which decides nothing here.
- electron-builder's msi target on the pinned version, and how a WiX fragment
  is attached to it.
- The WiX toolset the runner image carries, against what the target wants.
- The shim is still in `crates/desktop` and is stage 08's to remove; this
  stage leaves the crate alone.

[ADR-0020]: ../../adr/0020-electron-desktop.md
[releasing.md]: ../../releasing.md
