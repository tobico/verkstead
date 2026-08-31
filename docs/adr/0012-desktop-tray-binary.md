# A desktop tray binary beside the CLI

Verkstead reaches the desktop as a system-tray app: a second binary, from a new
`crates/desktop`, that runs the server in-process and puts a tray icon over it —
**Open** (the default action, double-click where the platform has one) opens the
web UI in the default browser, **View Logs** opens the log file, **Launch on
Startup** registers the current executable with the platform's startup
mechanism, **Exit** quits. The UI opens automatically on launch unless
`--no-open` says otherwise. No webview and no second UI: the viewer is already
embedded in the server and already installable as a PWA, so the tray's whole job
is lifecycle — which keeps the binary a few MB over the CLI and the toolchain
pure Rust (`tray-icon` + `muda`), where Tauri would add a webview stack and
Electron a bundled Chromium.

This revises [ADR-0004](0004-single-binary-distribution.md) from one binary to
**two single-file artifacts**: the headless `verkstead` CLI ships exactly as it
does today — the daemon install, the nix flake and the NixOS module all keep it
— and the desktop binary is a separate crate rather than a cargo feature, so the
CLI never carries GUI dependencies and neither artifact can half-include the
other. Everything ADR-0004 argued still holds per artifact: each is one file,
downloaded whole, unable to version-skew against itself.

The desktop app is deliberately plain about its limits:

- **Sessions stay Linux-only for now.** Sandboxing is bubblewrap and the PTY is
  `rustix`; the Windows and macOS builds cfg-gate the session machinery out and
  the UI says plainly that sessions need Linux. Porting is later work, platform
  by platform — shipping a clear notice was chosen over waiting for the ports or
  shipping remote-client apps, so the product is one thing everywhere it runs.
- **A taken port is an error.** If `127.0.0.1:8422` is already bound — a second
  copy, or the NixOS-module daemon — the app shows an error dialog and exits
  rather than fronting the running server or picking another port.
- **Unsigned on macOS and Windows.** The Gatekeeper approval dance is
  documented; SmartScreen's "run anyway" is left to the reader. Signing is a
  cost decision to revisit, not an architectural one.
- **Update checking gains nothing.** The server's daily poll and the viewer's
  banner (public-release stage 05) already cover the desktop; the tray adds no
  notifier and no self-update.

Packaging: a Windows portable exe, a universal (`lipo`) macOS app bundle in a
dmg, and a Linux x86_64 AppImage, each built by its own leg of the release
workflow beside the four bare-CLI legs. **Flatpak is deferred**: Flatpak's own
sandbox refuses the nested namespaces bubblewrap needs, so a working Flatpak
must run the bundled server on the host through `flatpak-spawn --host` — a
workaround stack not worth shipping in the first release. When demand appears it
ships as a `.flatpak` bundle on GitHub releases, not Flathub. The nix flake
stays daemon-only. The app identifier everywhere is `net.tobico.Verkstead`.

With a tray app launched from an icon rather than a shell, two defaults that
assumed a terminal move too: the Data Directory's default becomes the platform
data dir (`~/.local/share/verkstead`, `~/Library/Application Support/Verkstead`,
`%APPDATA%\Verkstead`) instead of the working directory — for `verkstead serve`
as much as for the desktop binary, one rule everywhere, breaking nobody because
nothing has been released — and the desktop binary writes the server's logs to a
small rotating file in the platform state/log dir, since stdout of a tray app
goes nowhere.

## Considered Options

- **Tauri or Electron** — a window of its own, at the cost of a webview stack
  (WebKitGTK on Linux) or a bundled Chromium, for a UI the browser already
  renders from localhost.
- **One binary with a cargo feature** — keeps ADR-0004's letter, but the
  headless artifact and the GUI artifact differ anyway, and a feature flag
  invites the CLI build to grow GUI dependencies by accident.
- **Remote-client apps on Windows/macOS** — honest about sessions being
  Linux-only, but makes the desktop app a different product per platform, and
  the tailnet PWA already covers remote use.
- **Fronting an already-running server instead of erroring** — single-instance
  behaviour for free, rejected for conflating the desktop app's server with a
  daemon of a different version and data directory.
