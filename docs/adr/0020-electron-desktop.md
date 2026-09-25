# An Electron desktop app over a sidecar server

Supersedes [ADR-0012](0012-desktop-tray-binary.md): the desktop app is a
window rather than a tray over a browser, and it is not written in Rust.

ADR-0012 put Verkstead on the desktop as lifecycle alone — a tray icon over
an in-process server, with the browser the human already had as the whole of
the interface — and rejected Electron and Tauri by name for the weight they
add to a UI the browser renders from localhost. What that bought was a few
megabytes over the CLI and a pure-Rust toolchain. What it cost turned out to
be the desktop itself: the workbench lived in a browser tab beside everything
else, the tray was four items that a tab cannot be, and every platform's
toolkit had to be spoken to by hand — GTK, AppKit and Win32 for two dialogs
and a menu, a StatusNotifierItem implementation of Verkstead's own after the
library one drew a menu on COSMIC that activated nothing. So the desktop app
becomes **an Electron application: a window of its own over the workbench,
the server run beside it as a sidecar, and the platform's tray, dialogs and
startup registration reached through one toolkit rather than three.**

Decided in the grilling of 2026-09-25. The roadmap that builds it is
`docs/roadmaps/electron-desktop/`.

## The sidecar is the CLI

**The app carries the headless `verkstead` binary and starts it as `verkstead
serve --desktop`.** Nothing of the desktop is in Rust any more: `crates/desktop`
goes, and with it the CLI's default-on `desktop` cargo feature and verb, the
Windows shim, the macOS launcher script and the AppImage's `AppRun`. The
binary is one slim artifact again on every platform, which is what
[ADR-0004](0004-single-binary-distribution.md) wanted before ADR-0012's
amendment had to bend it — and the headless CLI ships exactly as it does
today, as the separate package it always was.

The invariant that amendment guarded holds without effort. The Sandbox binds
the running server's own image first on a session's PATH so the two halves of
an ask cannot skew, and a sidecar that *is* the CLI is that image: every
session started under the app asks with the very binary that serves it. What
the app bundles is the CLI artifact the release already built — the static
musl binary on Linux, the two Mac builds joined with `lipo`, the Windows exe —
downloaded from the same run rather than compiled a second time. On Linux that
removes the glibc floor the AppImage carried: nothing in the sidecar links a
toolkit now, so the only floor is Electron's own.

**The flag is what the server knows about who started it**, and it changes
two things. The startup line names the address alone, because the log under
the app is a file on somebody's desk that a menu item opens
([ADR-0015](0015-open-boundary-and-workbench-key.md), as amended). And a
refused `tailscale serve` raises the platform's own password dialog rather
than handing back a `sudo` line: the three arms of that asking — `pkexec`,
`osascript`, PowerShell's `Start-Process -Verb RunAs` — were never anything
but a spawned command, so they move into the server crate as they are and the
flag is what turns them on. Nothing about the flag reaches the wire: the
viewer learns it is inside the app another way, below.

**Electron holds the key the way the tray app did.** The Workbench Key is
made by the server at first start and kept in the Data Directory; the app
waits for the server's health, reads the file, and opens its window on the
login link, so the window lands logged in. A 401 on the window's own frame is
the key having been reset from the phone, and the app reads the file again
and reloads. The address stays `127.0.0.1:8422` and a foreign listener there
is still a dialog and an exit, never a server fronted or a port picked; what
Electron adds is a single-instance lock, so a second launch of the *app* is
the window brought forward rather than that dialog. The server ending is the
app quitting. And the log file is the app's to write: the sidecar logs to
stdout as `verkstead serve` always has, and the app puts those lines beside
its own in `verkstead.log` under the Log Directory, with the byte-order mark
and the roll at a few megabytes the Rust app gave it.

## The window

**One window, decorated by the page.** It loads the workbench off the
loopback with nothing about the viewer changed to draw inside it, and it is
frameless: the platform's own controls stay — the traffic lights inset to
the left of the Wordmark on a Mac, the controls overlay at the top-right
corner on Windows and Linux, coloured from the page and recoloured when the
theme flips — and the pane heads are the drag region, their buttons excepted.
The frame is what keeps the outermost heads clear of the controls, reading the
overlay's rectangle, which is also what a narrow window's single pane gets. A
page with no pane head — onboarding, the no-such-page — is given a bare drag
bar of the same height, inside the app only. The menu bar is hidden on
Windows and Linux with its shortcuts kept; a Mac keeps the application menu it
cannot do without. Links off the workbench open in the system browser, and
the window never navigates away from localhost. Its size and position are
remembered between runs.

**Closing the window is a choice, and the choice is one control.** The Brief
named close-to-tray and warn-before-closing as two checkboxes that exclude
each other, which is a radio: **When the window is closed** — keep running in
the tray, ask before quitting, or quit — with keep running the default. Beside
it stand **Show tray icon**, on by default, and **Launch on Startup**. Turning
the tray off greys the tray position and the choice falls to Quit: a hidden
app with no icon is a Verkstead nobody can reach. The warning is the close
button's alone — the tray's Quit and Cmd+Q quit at once, because somebody who
chose Quit has already said what they meant. The tray menu is Open, View Logs
and Quit, and a left click on the icon opens the window.

**View Logs is the page's as well as the tray's.** A switch somebody can turn
off cannot be the only way to a log file, and the desktop most likely to have
it turned off is the one the tray misbehaved on — which is exactly the machine
whose log is worth reading. So the Desktop page carries a **View Logs** button
beside the switches, on every platform, opening the file the tray item opens;
the tray keeps its item for as long as there is a tray.

**A Mac is the platform's own.** The app is a regular Dock app there, the
menu-bar-only policy the tray app asked for gone: closing the window leaves
the app in the Dock, and a Dock click brings the window back. So the close
radio has nothing to choose on a Mac and is not drawn, and neither is the
warning; the Desktop page there holds the menu bar icon — on by default, one
rule everywhere — View Logs beside it, and Launch on Startup.

**A login start comes up hidden while the tray is shown**, the way `--no-open`
kept a login from being handed a browser window, and shown when it is not.
The registration is still the state rather than a copy of it: Electron's
login-item API on a Mac and Windows — the app is always a bundle now, which is
what the tray app's hand-written plist was working around — and an XDG
autostart file on Linux, where Electron has no such API.

**And the tray app's own registration is taken over rather than orphaned.**
Linux needs nothing for this, the entry being the same file under the same
name; the other two change mechanism, and a registration the new API cannot
see is one that goes on firing at a path this roadmap deletes while the box
above it reads off. So the first launch after an upgrade reads what the tray
app wrote — the launch agent at
`~/Library/LaunchAgents/net.tobico.Verkstead.plist`, the Run value named
`net.tobico.Verkstead` — carries whether it was on into the new registration,
and deletes it. Once, at launch, and never as a control: it is this app's own
registration under an older name rather than a second setting.

## The Desktop page

**Desktop settings are the app's, not the server's.** They are kept in a
JSON file of Electron's own under its user data, and reached from the page
over a preload bridge that exposes the platform, the settings, and the
startup registration. The Desktop page stands at the top of the settings and
is drawn only where the bridge exists — so a phone on the tailnet never sees
it, and nothing new is on the wire or in `config.yaml`. That is the split the
per-device push switch already made: a fact about the machine in front of the
human, never sent to the server.

## The tray on Linux, as a risk

Electron's tray speaks StatusNotifierItem through Chromium's own D-Bus code
rather than through the library the Rust app moved off, and Chromium watches
for the panel's name, so a panel that arrives after the app at login should
still get the icon. None of that is verifiable until it is built. The
decision is to accept it: the stage that builds the tray proves it on COSMIC
and against a late panel, and **Show tray icon** off is the way out on a
desktop that misbehaves.

## Packaging

**A top-level `desktop/` pnpm project beside `web/`, built by
electron-builder** into the same three artifacts, unsigned as before: a
universal dmg, an x86_64 AppImage, and an msi through electron-builder's WiX
target with a fragment of our own adding the install directory to the user's
PATH, which today's msi does and the adoption docs promise. Each release leg
launches the packed app and asserts what it asserted of the tray app — it
serves, the binary inside answers `ask`, and its log says the window and the
tray came up. The main process's pure parts are under vitest, and so is the
page with a stubbed bridge. nixpkgs' Electron joins the dev shell for running
the app locally; the flake stays daemon-only.

## Considered Options

- **Keeping `verkstead desktop` beside Electron**, or shrinking the crate to
  what Electron cannot do. Rejected: two desktop products, and nothing left
  that Electron cannot do.
- **Plain `verkstead serve` with Electron filtering the key out of the log**
  and the grant left as a `sudo` line. Rejected for a regex over a log line
  standing between a secret and a file on a desk.
- **The sidecar printing the login link on a machine-readable line**, or
  Electron generating the key and handing it in. Rejected: the file is the
  documented thing, and the server already makes it.
- **A free port when 8422 is taken.** Rejected as ADR-0012 rejected it: the
  CLI's default and `tailscale serve` both name the port.
- **Custom window controls drawn by the page.** Rejected: native controls
  keep Windows' snap layouts and a Mac's traffic lights as the platform draws
  them.
- **Desktop settings in `config.yaml` behind a server flag.** Rejected: a
  setting the server never enacts, put on the wire so a phone could flip it.
- **Three checkboxes as the Brief worded them.** Rejected for the radio: two
  boxes that exclude each other are one control.
- **An NSIS installer** in place of the msi. Rejected by the human: the msi
  stays.
- **Electron Forge**, or the project as a package inside `web/`. Rejected for
  electron-builder's three targets in one configuration, and for a project
  that consumes a built binary rather than sharing a toolchain.
