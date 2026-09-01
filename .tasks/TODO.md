# 02. The desktop crate and the Linux tray

`cargo run -p verkstead-desktop` on Linux puts a Verkstead icon in the system
tray, runs the server in-process on `127.0.0.1:8422` out of the platform Data
Directory, and opens the viewer in the default browser. The menu is **Open**,
**View Logs**, **Launch on Startup** and **Exit**, and that is the whole of the
app: no webview and no second UI, because the viewer is already embedded in the
server and already installable as a PWA, so the tray's whole job is lifecycle
(ADR-0012). `--no-open` suppresses the auto-open, a taken port shows a dialog
naming it and exits, and the server's `tracing` goes to a rotating file in the
**Log Directory** rather than to a stdout nobody launched from an icon will
ever read.

A new crate rather than a feature of the CLI: `crates/desktop` links
`verkstead-server` the way `crates/cli` does, so the headless binary carries no
GUI dependency and ships exactly as it does today. On Linux the tray is GTK3
and libappindicator, which is the first thing in this repository to need system
GUI libraries — so the CI runner and the dev shell both grow them with the
crate rather than after it, and the answer is the one stage 03 puts inside the
AppImage.

Roadmap stage: [02: The desktop crate and the Linux tray](docs/roadmaps/desktop/02-desktop-crate.md)

## Tasks

- [x] 01: The crate, the server in-process, and the browser — [details](01-crate-server-and-browser.md)
- [x] 02: The tray icon, Open and Exit — [details](02-tray-open-and-exit.md)
- [ ] 03: The rotating log file, and View Logs — [details](03-log-file-and-view-logs.md)
- [ ] 04: Launch on Startup — [details](04-launch-on-startup.md)
