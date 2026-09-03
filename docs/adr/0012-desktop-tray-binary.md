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

Amended: **the artifacts keep their builds and lose their separate grammars —
`verkstead` is the one binary again, and the tray is its `desktop` verb.** What
ADR-0004 guarded against arrived by a door this ADR did not watch: the sandbox
hands every session the running server's own image, bound first on the
session's PATH so the two halves of an ask cannot skew — and once the server
moved in-process here, that image was a desktop binary with no `ask` in it at
all. So the desktop crate becomes a library behind a default-on `desktop`
cargo feature of the CLI, and every image that can serve can also ask. The
headless artifact still ships exactly as it does today: the static musl CLI is
built with the feature off, which is also what now polices GUI dependencies
creeping into it — GTK will not link there, so the creep this paragraph feared
fails that leg loudly instead of arriving by accident. The nix daemon package
builds the same slim way. What stays two files is packaging rather than
grammar: the launchers that cannot say a verb get a shim — a windows-subsystem
exe beside the CLI, a launcher script naming the bundle's executable on a Mac
— and the Windows download becomes an msi carrying both, a shim beside its
binary never having been one portable file. And the server stops trusting the
invariant it stands on: at startup it probes its own image with `guide`, in
the environment a session would get rather than its own, and a failed probe
refuses sessions the way a missing image already does.

The desktop app is deliberately plain about its limits:

- **Sessions stay Linux-only for now.** Sandboxing is bubblewrap and the PTY is
  `rustix`; the Windows and macOS builds cfg-gate the session machinery out and
  the UI says plainly that sessions need Linux. Porting is later work, platform
  by platform — shipping a clear notice was chosen over waiting for the ports or
  shipping remote-client apps, so the product is one thing everywhere it runs.

  Amended: **macOS is the first of those ports, and it lands on
  `sandbox-exec`.** What a session may reach is now one description said once
  and rendered twice — bubblewrap's flags on Linux, a deny-by-default policy on
  a Mac — so a Mac runs the product whole rather than the product minus
  sessions. The two renderings are not the same boundary, and the difference is
  accepted rather than papered over: **Apple's denies where bubblewrap hides.**
  A session on Linux is in a mount namespace where the rest of the machine is
  simply not there; a session on a Mac can see the machine and is refused it,
  and every path that exists only inside a bind on Linux — a fresh HOME, the
  account mounted into it, the skills, the binary a session asks with — has to
  be a real path there. What is inside is the same either way, which is the
  whole of what the description is for.

  With one exception, and it is the one place a Mac session reaches **more**
  than a Linux one: **`/tmp` is the machine's own there.** On Linux it is a
  filesystem of the session's own, holding nothing of the host's and gone when
  the session is; a policy has no such thing to offer, so on a Mac it is the
  real `/private/tmp` — what anything else on the machine left there is
  readable, and what a session writes stays behind. Accepted rather than
  narrowed: a session's temporary directory of its own would mean every tool
  reaching for the literal `/tmp` refused, which is most of them. The
  Conversation's handoff directory is the one thing that was under `/tmp` and
  is not: every Conversation on the machine would have been sharing it.

  And `sandbox-exec` is **deprecated by Apple, with no replacement an unsigned
  app can use**: the supported way to sandbox is an entitlement on a signed
  bundle, applied to the app itself rather than to a child it spawns, and this
  app is unsigned by the decision below. The command has been deprecated since
  macOS 10.10, is what Apple's own tooling still runs behind, and there is no
  sign of it going. A risk taken with open eyes: the day it goes, Mac sessions
  go with it until something replaces them.

  Windows is untouched by any of this. The UI state the bullet above promised
  — the one saying a session cannot be started here — is not something this
  port built either: it stayed a decision with nothing behind it until the
  Windows port, which is the stage that built it.

  Amended: **the Windows port gates at the leaf call sites rather than around
  the modules.** Measured against the tree when that port was planned, the code
  that will not compile there is the pseudo-terminal and four call sites under
  it; everything above them — the sessions module, the runner, the Screen, the
  Capture, the transcript readers — is ordinary portable Rust. So it goes on
  being built there, and what stands between a Windows Verkstead and a session
  it cannot run is one honest refusal where a session would start. `HOME` and
  the Build Cache resolve there from `%USERPROFILE%` and `%LOCALAPPDATA%`
  rather than being skipped on a platform that runs none: both are what a
  session will want when one arrives. And the state says **Windows**, and says
  **not yet** — a Mac runs sessions now, and a stage after the port brings them
  to Windows as well.
- **A taken port is an error.** If `127.0.0.1:8422` is already bound — a second
  copy, or the NixOS-module daemon — the app shows an error dialog and exits
  rather than fronting the running server or picking another port.
- **Unsigned on macOS and Windows.** The Gatekeeper approval dance is
  documented; SmartScreen's "run anyway" is left to the reader. Signing is a
  cost decision to revisit, not an architectural one.

  Amended: **SmartScreen's is written out as well.** The stage that shipped the
  exe put its two clicks beside the download in `docs/adoption.md`, where
  Gatekeeper's three already were: a reader stopped by a blue window whose only
  button says *Don't run* is not one to leave to work it out. What stands is the
  decision to ship unsigned; what changed is that both platforms' way past it is
  now written down.
- **Update checking gains nothing.** The server's daily poll and the viewer's
  banner (public-release stage 05) already cover the desktop; the tray adds no
  notifier and no self-update.

Packaging: a Windows portable exe, a universal (`lipo`) macOS app bundle in a
dmg, and a Linux x86_64 AppImage, each built by its own leg of the release
workflow beside the bare-CLI legs — four of those until the Windows port, which
adds a fifth for the same reason it is nearly free: once the server compiles
there, so does the CLI. **Flatpak is deferred**: Flatpak's own sandbox refuses
the nested namespaces bubblewrap needs, so a working Flatpak must run the
bundled server on the host through `flatpak-spawn --host` — a workaround stack
not worth shipping in the first release. When demand appears it ships as a
`.flatpak` bundle on GitHub releases, not Flathub. The nix flake stays
daemon-only. The app identifier everywhere is `net.tobico.Verkstead`.

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
