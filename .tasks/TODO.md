# A download offers to install itself

A packed Verkstead run from outside an Applications folder — the dmg or
Downloads on a Mac, wherever the AppImage was saved on Linux — puts up one
native dialog before its sidecar starts: *Install Verkstead?*, **Install**,
**Skip**, **Don't ask again**. Install moves the app into Applications
(`/Applications` through Electron's own call on a Mac; `~/Applications` with a
desktop entry and the icon set on Linux) and relaunches it from there, so the
Startup Registration and the Update both point at a file that stays. A newer
Verkstead already installed is offered to open instead. The Desktop section of
the settings carries the same Install as a row while the app runs from outside
Applications, and `--install` is that Install with nobody asked, which is how
the release legs prove it. Windows is untouched: its download is the installer.

Settled in the grilling of 2026-10-29 (Sets 1515 to 1518) and recorded in
ADR-0020, *Amended: a download offers to install itself*, which is the decision
record every task below answers to; `CONTEXT.md`'s **Install Offer** is the
vocabulary.

## Tasks

- [x] 01: The offer, on Linux — [details](01-the-offer-on-linux.md)
- [x] 02: The offer, on a Mac — [details](02-the-offer-on-a-mac.md)
- [ ] 03: A newer Verkstead is already installed — [details](03-a-newer-verkstead-is-already-installed.md)
- [ ] 04: Install from the Desktop page — [details](04-install-from-the-desktop-page.md)
- [ ] 05: Proven in the release — [details](05-proven-in-the-release.md)
