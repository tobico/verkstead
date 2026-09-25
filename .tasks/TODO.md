# Tray, close policy and the Desktop page

The app puts an icon in the tray with **Open**, **View Logs** and **Quit**, and
closing the window becomes a choice rather than a quit: keep running in the tray,
which is the default, ask before quitting, or quit. A new **Desktop** section
stands at the top of the settings — drawn only inside the app, over a preload
bridge, out of a JSON file of Electron's own — holding that choice, **Show tray
icon**, **Launch on Startup** and a **View Logs** button opening the file the
tray item opens. Turning the tray off greys the tray position and the choice
falls to Quit, because a hidden app with no icon is a Verkstead nobody can reach;
the tray's own Quit never asks. On Linux **Launch on Startup** is an XDG
autostart entry under the name the Rust tray app already writes, and a login
start comes up hidden while the tray is shown.

Nothing new goes on the wire or into `config.yaml`: the desktop settings are a
fact about the machine in front of the human, which is the split the per-device
push switch already made. The window is still the platform's own decorated
window — the frameless one is stage 04's — and packaging is 05's onwards, so
every run in this stage is `pnpm start` from the dev shell. The Linux tray is
Electron's own and accepted as a risk, which this stage discharges on COSMIC and
against a panel that arrives after the app. The decisions are in
[ADR-0020](docs/adr/0020-electron-desktop.md).

Roadmap stage: [03: Tray, close policy and the Desktop page](docs/roadmaps/electron-desktop/03-tray-and-desktop-page.md)

## Tasks

- [x] 01: The tray — [details](01-the-tray.md)
- [x] 02: The close policy, over the app's own settings — [details](02-the-close-policy.md)
- [x] 03: The bridge — [details](03-the-bridge.md)
- [ ] 04: The Desktop page — [details](04-the-desktop-page.md)
- [ ] 05: Launch on Startup, and the hidden login start — [details](05-launch-on-startup.md)
- [ ] 06: COSMIC and the late panel — [details](06-cosmic-and-the-late-panel.md)
- [ ] 07: The words — [details](07-the-words.md)
