# 02. The desktop crate and the Linux tray

## Goal

`cargo run -p verkstead-desktop` on Linux puts a Verkstead icon in the system
tray, runs the server in-process on `127.0.0.1:8422`, and opens the viewer in
the default browser. The menu is **Open** (the default action, double-click
where the platform has one), **View Logs**, **Launch on Startup** (a checkbox),
**Exit**. `--no-open` suppresses the auto-open; a taken port shows an error
dialog and exits; server logs land in a rotating file under the stage-01
state/log dir.

## Decisions in force

- **Rust-native tray, browser UI** ([ADR-0012], grilling Q2): `tray-icon` +
  `muda`, no webview and no second UI — the viewer is embedded in the server
  and already installable as a PWA. Tauri and Electron were rejected for the
  weight they add to a UI the browser already renders.
- **A new crate and a second binary** (Q2a): `crates/desktop`, linking
  `verkstead-server` the way `crates/cli` does. The CLI carries no GUI
  dependency and ships unchanged.
- **A taken port is an error** (Q5): dialog naming the port, then exit. No
  fronting a running server, no fallback port — both rejected in the ADR.
- **Launch on Startup re-registers on every launch** (Q6): while the box is
  checked, each launch rewrites the platform startup entry with the current
  executable path, so a moved binary heals itself; the platform registration
  (XDG autostart file here; Run key and plist in stages 04/05) is the source of
  truth the checkbox reads — no config entry duplicates it.
- **File logs plus a way to read them** (Q8): a tray app's stdout goes
  nowhere, so the desktop binary routes `tracing` to a small rotating file, and
  **View Logs** opens it with the platform opener. The plain CLI keeps logging
  to stdout.
- The flag is `--no-open`; the app id everywhere is `net.tobico.Verkstead`.
- First-run is the browser's job: the settings page already edits Watched
  Paths, so the tray adds no configuration UI.

## Proposed tasks (provisional)

1. **Crate, in-process server, auto-open** — `crates/desktop` boots the server
   with the platform data dir, opens the browser, honours `--no-open`, and
   fails the taken port with a dialog. Accepts: server reachable at 8422 after
   launch; `--no-open` opens nothing; a bound port exits nonzero after the
   dialog.
2. **Tray menu, Open, Exit** — icon from `assets/icons/`, menu wired, Open as
   the default/double-click action, Exit shuts the server down cleanly.
   Accepts: manual run on this machine plus whatever headless test the tray
   libs allow.
3. **File logging and View Logs** — rotating file in the state/log dir, View
   Logs opens it. Accepts: file exists and rotates by size; menu item opens it.
4. **Launch on Startup** — checkbox reads the XDG autostart entry, checking
   writes it, every launch refreshes it while present. Accepts: entry appears
   and disappears with the checkbox; a stale path is rewritten on launch.
5. **The CI job's system packages** — lands with task 1 rather than after
   it: `ci.yml` builds the whole workspace (`cargo clippy --workspace
   --all-targets -- -D warnings`, `cargo test --workspace`) on a runner that
   installs bubblewrap and nothing else, so the tray's GTK and appindicator
   development packages have to be there the first time `crates/desktop` is
   pushed. Accepts: CI is green on the branch that adds the crate, with the
   packages installed by a step rather than by the runner image happening to
   carry them.

## Re-verify at start

- `verkstead_server::run` blocks with no shutdown handle
  (`crates/server/src/lib.rs`) — the tray needs a clean-shutdown path; decide
  its shape against the code as it stands then.
- Which tray backend `tray-icon` uses on Linux (appindicator vs ksni), what
  GTK/event-loop it demands, and what that means for the CI runner here and
  for the AppImage in stage 03 — the same packages answer both, and this stage
  is where the question is first forced.
- Stage 01 landed: the platform data-dir and state/log-dir helpers exist.
- Double-click default actions are a Windows/Linux affordance; macOS opens the
  menu on click — "where possible" per the Brief, no workaround owed.

[ADR-0012]: ../../adr/0012-desktop-tray-binary.md
